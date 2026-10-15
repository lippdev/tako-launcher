using System.IO;
using System.Text.RegularExpressions;
using System.Xml;

namespace Launcher.Themes.Xaml;

/// <summary>
/// Valida XAML de tema como XML puro, antes de qualquer interpretação pelo WPF (MVP-016).
/// Não instancia tipos nem carrega assemblies: só compara nomes contra a <see cref="XamlSecurityPolicy"/>.
/// </summary>
public sealed partial class XamlValidator(XamlSecurityPolicy policy)
{
    public XamlSecurityPolicy Policy => policy;

    private const string XmlNamespace = "http://www.w3.org/XML/1998/namespace";
    private const string XmlnsNamespace = "http://www.w3.org/2000/xmlns/";
    private const string AllowedPackPrefix = "pack://application:,,,/";
    private static readonly char[] PathSeparators = ['/', '\\'];

    private static readonly HashSet<string> XamlDirectiveAttributes = new(StringComparer.Ordinal) { "Key", "Name", "Shared", "Uid" };
    private static readonly HashSet<string> XamlElements = new(StringComparer.Ordinal) { "Null", "Type", "Static", "Array", "Reference" };

    /// <summary>
    /// Atributos aceitos no <c>Popup</c> da lista suspensa. Ficam de fora posição livre (offsets, PlacementTarget,
    /// PlacementRectangle), StaysOpen e qualquer outra forma de abrir ou mover a janela do Popup.
    /// </summary>
    private static readonly HashSet<string> PopupAttributes = new(StringComparer.Ordinal)
    {
        "IsOpen", "Placement", "Focusable", "AllowsTransparency", "PopupAnimation", "SnapsToDevicePixels",
    };

    /// <summary>Propriedades do <c>Popup</c> que gatilhos podem alterar por <c>Setter TargetName</c>.</summary>
    private static readonly HashSet<string> PopupSetterProperties = new(StringComparer.Ordinal)
    {
        "Focusable", "AllowsTransparency", "PopupAnimation", "SnapsToDevicePixels",
    };

    /// <param name="xaml">Conteúdo do arquivo.</param>
    /// <param name="themeRelativePath">Caminho do arquivo dentro do tema (ex.: <c>Views/Main.xaml</c>), usado para detectar path traversal.</param>
    /// <param name="expectedRoot">Elemento raiz exigido no namespace de apresentação (ex.: <c>ResourceDictionary</c>), ou nulo.</param>
    public XamlValidationResult Validate(string xaml, string themeRelativePath, string? expectedRoot = null)
    {
        ArgumentNullException.ThrowIfNull(xaml);
        ArgumentNullException.ThrowIfNull(themeRelativePath);

        var violations = new List<XamlViolation>();
        if (xaml.Length > policy.MaxCharacters)
        {
            violations.Add(new(XamlViolationCode.LimitExceeded, $"Arquivo excede {policy.MaxCharacters} caracteres.", 0, 0));
            return new(violations);
        }

        var settings = new XmlReaderSettings
        {
            DtdProcessing = DtdProcessing.Prohibit,
            XmlResolver = null,
            MaxCharactersInDocument = policy.MaxCharacters,
            IgnoreComments = true,
            IgnoreWhitespace = true,
        };

        using var reader = XmlReader.Create(new StringReader(xaml), settings);
        var walker = new Walker(policy, reader, themeRelativePath, expectedRoot, violations);
        IReadOnlyList<UnusedNamespaceDeclaration> unused = [];
        try
        {
            walker.Run();
            walker.FinishPopupTargets();
            unused = walker.FinishNamespaces();
        }
        catch (XmlException ex) when (ex.Message.Contains("DTD", StringComparison.OrdinalIgnoreCase))
        {
            violations.Add(new(XamlViolationCode.DtdProhibited, "DTD não é permitido em XAML de tema.", ex.LineNumber, ex.LinePosition));
        }
        catch (XmlException ex)
        {
            violations.Add(new(XamlViolationCode.MalformedXml, ex.Message, ex.LineNumber, ex.LinePosition));
        }

        return new(violations, unused);
    }

    [GeneratedRegex(@"\{\s*(?:(?<prefix>[A-Za-z_][\w\-.]*):)?(?<name>[A-Za-z_][\w.]*)(?<args>[^{}]*)")]
    private static partial Regex MarkupExtensionPattern();

    [GeneratedRegex(@"^[A-Za-z]:[\\/]")]
    private static partial Regex DriveLetterPath();

    [GeneratedRegex(@"^\{\s*TemplateBinding\s+(?:Property\s*=\s*)?(?:ComboBox\.)?IsDropDownOpen\s*\}$")]
    private static partial Regex DropDownOpenBinding();

    [GeneratedRegex(@"^\{\s*(?:[\w\-.]+:)?Type(?:Extension)?\s+(?:TypeName\s*=\s*)?(?<type>[\w:.]+)\s*\}$")]
    private static partial Regex TypeExtensionValue();

    private enum ElementKind
    {
        Plain,
        Template,
        PopupHost,
        Popup,
    }

    private sealed class Walker(
        XamlSecurityPolicy policy,
        XmlReader reader,
        string themeRelativePath,
        string? expectedRoot,
        List<XamlViolation> violations)
    {
        private readonly IXmlLineInfo _lineInfo = (IXmlLineInfo)reader;
        private readonly Stack<string> _elements = new();
        private readonly Stack<ElementKind> _kinds = new();
        private readonly HashSet<string> _popupNames = new(StringComparer.Ordinal);
        private readonly List<(string Target, string? Property, int Line, int Column)> _targeted = [];
        private readonly Stack<bool> _templates = new();
        private int _popupDepth;
        private readonly Dictionary<string, (Attribute Declaration, bool Used)> _disallowed = new(StringComparer.Ordinal);
        private int _elementCount;

        public void Run()
        {
            while (reader.Read())
            {
                switch (reader.NodeType)
                {
                    case XmlNodeType.Element:
                        VisitElement();
                        break;
                    case XmlNodeType.EndElement:
                        _elements.Pop();
                        Leave(_kinds.Pop());
                        break;
                    case XmlNodeType.Text:
                    case XmlNodeType.CDATA:
                        VisitText();
                        break;
                    case XmlNodeType.ProcessingInstruction:
                        Add(XamlViolationCode.DirectiveNotAllowed, $"Instrução de processamento '{reader.Name}' não é permitida.");
                        break;
                }
            }
        }

        private void VisitElement()
        {
            if (++_elementCount > policy.MaxElements)
            {
                Add(XamlViolationCode.LimitExceeded, $"Arquivo excede {policy.MaxElements} elementos.");
                throw new XmlException("Limite de elementos excedido.");
            }

            if (reader.Depth >= policy.MaxDepth)
            {
                Add(XamlViolationCode.LimitExceeded, $"Aninhamento excede {policy.MaxDepth} níveis.");
                throw new XmlException("Limite de profundidade excedido.");
            }

            string ns = reader.NamespaceURI;
            string name = reader.LocalName;
            int line = _lineInfo.LineNumber;
            int column = _lineInfo.LinePosition;

            if (_elementCount == 1 && expectedRoot is not null
                && (ns != XamlSecurityPolicy.PresentationNamespace || name != expectedRoot))
            {
                Add(XamlViolationCode.UnexpectedRoot, $"Elemento raiz deve ser '{expectedRoot}', encontrado '{reader.Name}'.");
            }

            var attributes = ReadAttributes();
            foreach (var attribute in attributes.Where(a => a.IsNamespaceDeclaration))
            {
                ValidateNamespaceDeclaration(attribute);
            }

            var kind = ElementKind.Plain;
            bool presentation = ns == XamlSecurityPolicy.PresentationNamespace;
            if (presentation && name == "Popup")
            {
                ValidatePopup(attributes, line, column);
                kind = ElementKind.Popup;
            }
            else
            {
                ValidateElementName(ns, name, reader.Name, line, column);
                if (presentation && name is "ControlTemplate" or "DataTemplate" or "HierarchicalDataTemplate" or "ItemsPanelTemplate")
                {
                    kind = name == "ControlTemplate" && policy.PopupHostTemplates.Contains(TargetTypeOf(attributes) ?? string.Empty)
                        ? ElementKind.PopupHost
                        : ElementKind.Template;
                }
            }

            if (TargetTypeOf(attributes) == "Popup")
            {
                // Estilo ou template implícito para Popup alcançaria os Popups permitidos sem as restrições abaixo.
                Add(XamlViolationCode.TypeNotAllowed, "Estilos e templates com TargetType Popup não são permitidos.", line, column);
            }

            RecordTargetName(presentation ? name : null, attributes);
            foreach (var attribute in attributes.Where(a => !a.IsNamespaceDeclaration))
            {
                ValidateAttribute(ns, name, attribute, attributes);
            }

            if (reader.IsEmptyElement)
            {
                return;
            }

            _elements.Push(name);
            _kinds.Push(kind);
            Enter(kind);
        }

        private void Enter(ElementKind kind)
        {
            switch (kind)
            {
                case ElementKind.Template or ElementKind.PopupHost:
                    _templates.Push(kind == ElementKind.PopupHost);
                    break;
                case ElementKind.Popup:
                    _popupDepth++;
                    break;
            }
        }

        private void Leave(ElementKind kind)
        {
            switch (kind)
            {
                case ElementKind.Template or ElementKind.PopupHost:
                    _templates.Pop();
                    break;
                case ElementKind.Popup:
                    _popupDepth--;
                    break;
            }
        }

        /// <summary>
        /// Popup só como lista de uma lista suspensa: dentro do template de um tipo de <see cref="XamlSecurityPolicy.PopupHostTemplates"/>,
        /// sem Popup aninhado, aberto apenas por <c>IsDropDownOpen</c> e posicionado abaixo ou acima do controle.
        /// </summary>
        private void ValidatePopup(List<Attribute> attributes, int line, int column)
        {
            // O template mais interno precisa ser o da lista suspensa (não vale um template aninhado dentro dele).
            if (!_templates.TryPeek(out bool inHost) || !inHost || _popupDepth > 0)
            {
                Add(XamlViolationCode.TypeNotAllowed, "Tipo 'Popup' só é permitido dentro do template de uma lista suspensa (ComboBox).", line, column);
            }

            foreach (var attribute in attributes.Where(a => !a.IsNamespaceDeclaration))
            {
                string value = attribute.Value.Trim();
                if (attribute.LocalName == "Name" && attribute.Namespace is "" or XamlSecurityPolicy.XamlNamespace)
                {
                    _popupNames.Add(value);
                    continue;
                }

                bool allowed = attribute.Namespace.Length == 0 && attribute.LocalName switch
                {
                    "IsOpen" => DropDownOpenBinding().IsMatch(value) || value == "False",
                    "Placement" => value is "Bottom" or "Top",
                    _ => PopupAttributes.Contains(attribute.LocalName),
                };

                if (!allowed)
                {
                    Add(XamlViolationCode.TypeNotAllowed, $"Atributo '{attribute.QualifiedName}=\"{value}\"' não é permitido no Popup da lista suspensa.", attribute.Line, attribute.Column);
                }
            }
        }

        /// <summary>Guarda alvos de Setter/Storyboard por nome; os que apontam para um Popup são conferidos no fim.</summary>
        private void RecordTargetName(string? presentationElement, List<Attribute> attributes)
        {
            foreach (var attribute in attributes.Where(a => a.Namespace.Length == 0))
            {
                if (presentationElement == "Setter" && attribute.LocalName == "TargetName")
                {
                    string? property = attributes.FirstOrDefault(a => a.Namespace.Length == 0 && a.LocalName == "Property")?.Value;
                    _targeted.Add((attribute.Value.Trim(), property is null ? null : LastSegment(property.Trim()), attribute.Line, attribute.Column));
                }
                else if (attribute.LocalName is "Storyboard.TargetName" or "Storyboard.Target")
                {
                    // Animação de um Popup poderia movê-lo ou abri-lo; o nome é procurado também em {Binding ElementName=...}.
                    _targeted.Add((attribute.Value.Trim(), null, attribute.Line, attribute.Column));
                }
            }
        }

        public void FinishPopupTargets()
        {
            foreach (var (target, property, line, column) in _targeted)
            {
                string? popup = _popupNames.FirstOrDefault(n => target == n || Regex.IsMatch(target, $@"ElementName\s*=\s*{Regex.Escape(n)}\b"));
                if (popup is not null && (property is null || !PopupSetterProperties.Contains(property)))
                {
                    Add(XamlViolationCode.TypeNotAllowed, $"Gatilhos e animações não podem alterar '{property ?? "propriedades"}' do Popup '{popup}'.", line, column);
                }
            }
        }

        private static string? TargetTypeOf(List<Attribute> attributes)
        {
            string? value = attributes.FirstOrDefault(a => a.Namespace.Length == 0 && a.LocalName == "TargetType")?.Value.Trim();
            if (value is null)
            {
                return null;
            }

            var match = TypeExtensionValue().Match(value);
            string type = match.Success ? match.Groups["type"].Value : value;
            return type[(type.IndexOf(':', StringComparison.Ordinal) + 1)..];
        }

        private List<Attribute> ReadAttributes()
        {
            var attributes = new List<Attribute>();
            if (reader.MoveToFirstAttribute())
            {
                do
                {
                    attributes.Add(new(
                        reader.NamespaceURI,
                        reader.Prefix,
                        reader.LocalName,
                        reader.Name,
                        reader.Value,
                        reader.NamespaceURI == XmlnsNamespace,
                        _lineInfo.LineNumber,
                        _lineInfo.LinePosition));
                }
                while (reader.MoveToNextAttribute());

                reader.MoveToElement();
            }

            return attributes;
        }

        private void ValidateNamespaceDeclaration(Attribute attribute)
        {
            string value = attribute.Value;
            bool allowed = value is XamlSecurityPolicy.PresentationNamespace
                or XamlSecurityPolicy.XamlNamespace
                or XamlSecurityPolicy.MarkupCompatibilityNamespace
                or XamlSecurityPolicy.DesignerNamespace
                or XamlSecurityPolicy.PresentationOptionsNamespace
                || policy.ClrNamespaces.ContainsKey(value);

            if (allowed)
            {
                return;
            }

            // Namespace padrão não permitido afetaria todo nome sem prefixo: rejeita já.
            if (attribute.Prefix.Length == 0)
            {
                Add(XamlViolationCode.NamespaceNotAllowed, $"Namespace '{value}' não é permitido.", attribute.Line, attribute.Column);
                return;
            }

            // Só declarar não carrega nada; vira violação apenas se for usado (ver FinishNamespaces).
            _disallowed[attribute.LocalName] = (attribute, false);
        }

        /// <summary>Declarações não permitidas: usadas viram violação; não usadas são devolvidas para remoção.</summary>
        public List<UnusedNamespaceDeclaration> FinishNamespaces()
        {
            var unused = new List<UnusedNamespaceDeclaration>();
            foreach (var (prefix, (declaration, used)) in _disallowed)
            {
                if (used)
                {
                    Add(XamlViolationCode.NamespaceNotAllowed, $"Namespace '{declaration.Value}' não é permitido.", declaration.Line, declaration.Column);
                }
                else
                {
                    unused.Add(new UnusedNamespaceDeclaration(prefix, declaration.Value));
                }
            }

            return unused;
        }

        private void MarkUsed(string? ns)
        {
            foreach (var (prefix, entry) in _disallowed)
            {
                if (entry.Declaration.Value == ns)
                {
                    _disallowed[prefix] = (entry.Declaration, true);
                }
            }
        }

        /// <summary>Prefixos não permitidos citados em valores (ex.: Setter Property="i:Interaction.Behaviors").</summary>
        private void CheckPrefixReferences(string value, int line, int column)
        {
            foreach (var (prefix, entry) in _disallowed)
            {
                if (Regex.IsMatch(value, $@"(?<![\w.]){Regex.Escape(prefix)}:"))
                {
                    _disallowed[prefix] = (entry.Declaration, true);
                    Add(XamlViolationCode.NamespaceNotAllowed, $"Valor usa o prefixo '{prefix}' de namespace não permitido.", line, column);
                }
            }
        }

        private void ValidateElementName(string ns, string name, string qualifiedName, int line, int column)
        {
            int dot = name.LastIndexOf('.');
            if (dot > 0)
            {
                // Elemento de propriedade: <Owner.Property>.
                if (!IsOwnerAllowed(ns, name[..dot]))
                {
                    Add(XamlViolationCode.TypeNotAllowed, $"Elemento de propriedade '{qualifiedName}' usa tipo não permitido.", line, column);
                }

                return;
            }

            switch (ns)
            {
                case XamlSecurityPolicy.PresentationNamespace when policy.PresentationTypes.Contains(name):
                    return;
                case XamlSecurityPolicy.PresentationNamespace:
                    Add(XamlViolationCode.TypeNotAllowed, $"Tipo '{name}' não é permitido em temas.", line, column);
                    return;
                case XamlSecurityPolicy.XamlNamespace when XamlElements.Contains(StripExtension(name)):
                    return;
                case XamlSecurityPolicy.XamlNamespace:
                    Add(XamlViolationCode.DirectiveNotAllowed, $"Diretiva '{qualifiedName}' não é permitida.", line, column);
                    return;
                case XamlSecurityPolicy.MarkupCompatibilityNamespace:
                    Add(XamlViolationCode.MarkupCompatibilityNotAllowed, $"Elemento '{qualifiedName}' de markup compatibility não é permitido.", line, column);
                    return;
            }

            if (policy.ClrNamespaces.TryGetValue(ns, out var types))
            {
                if (!types.Contains(name))
                {
                    Add(XamlViolationCode.TypeNotAllowed, $"Tipo '{qualifiedName}' não é permitido em temas.", line, column);
                }

                return;
            }

            MarkUsed(ns);
            Add(XamlViolationCode.NamespaceNotAllowed, $"Elemento '{qualifiedName}' está em namespace não permitido.", line, column);
        }

        private void ValidateAttribute(string elementNs, string elementName, Attribute attribute, List<Attribute> siblings)
        {
            switch (attribute.Namespace)
            {
                case XmlNamespace:
                case XamlSecurityPolicy.DesignerNamespace:
                case XamlSecurityPolicy.PresentationOptionsNamespace:
                    return;
                case XamlSecurityPolicy.XamlNamespace:
                    if (!XamlDirectiveAttributes.Contains(attribute.LocalName))
                    {
                        Add(XamlViolationCode.DirectiveNotAllowed, $"Diretiva '{attribute.QualifiedName}' não é permitida.", attribute.Line, attribute.Column);
                        return;
                    }

                    ValidateValue(attribute.LocalName, attribute.Value, attribute);
                    return;
                case XamlSecurityPolicy.MarkupCompatibilityNamespace:
                    ValidateMarkupCompatibility(attribute);
                    return;
            }

            string name = attribute.LocalName;
            int dot = name.LastIndexOf('.');
            if (dot > 0)
            {
                // Propriedade anexada: o dono é resolvido no namespace do prefixo (ou no namespace padrão).
                string ownerNs = attribute.Namespace.Length > 0 ? attribute.Namespace : reader.LookupNamespace(string.Empty) ?? string.Empty;
                if (!IsOwnerAllowed(ownerNs, name[..dot]))
                {
                    MarkUsed(ownerNs);
                    Add(XamlViolationCode.TypeNotAllowed, $"Propriedade anexada '{attribute.QualifiedName}' usa tipo não permitido.", attribute.Line, attribute.Column);
                    return;
                }
            }
            else if (attribute.Namespace.Length > 0)
            {
                MarkUsed(attribute.Namespace);
                Add(XamlViolationCode.NamespaceNotAllowed, $"Atributo '{attribute.QualifiedName}' está em namespace não permitido.", attribute.Line, attribute.Column);
                return;
            }

            string property = name[(dot + 1)..];
            if (elementNs == XamlSecurityPolicy.PresentationNamespace)
            {
                if (elementName == "ResourceDictionary" && property == "Source")
                {
                    Add(XamlViolationCode.NestedLoadNotAllowed, "ResourceDictionary.Source carregaria outro XAML sem validação; o motor de temas deve compor os dicionários.", attribute.Line, attribute.Column);
                    return;
                }

                if (elementName == "Setter" && property == "Value")
                {
                    // Setter Property="Source" Value="..." passa pelo conversor de URI da propriedade-alvo.
                    var target = siblings.FirstOrDefault(a => a.Namespace.Length == 0 && a.LocalName == "Property");
                    if (target is not null && policy.UriProperties.Contains(LastSegment(target.Value)))
                    {
                        property = LastSegment(target.Value);
                    }
                }
            }

            if (elementNs == XamlSecurityPolicy.XamlNamespace)
            {
                ValidateXamlElementArgument(elementName, property, attribute);
            }

            ValidateValue(property, attribute.Value, attribute);
        }

        private void ValidateXamlElementArgument(string elementName, string property, Attribute attribute)
        {
            switch (StripExtension(elementName), property)
            {
                case ("Static", "Member"):
                    ValidateStaticMember(attribute.Value.Trim(), attribute.Line, attribute.Column);
                    break;
                case ("Type", "TypeName"):
                case ("Array", "Type"):
                    ValidateTypeReference(attribute.Value.Trim(), attribute.Line, attribute.Column);
                    break;
            }
        }

        private void ValidateMarkupCompatibility(Attribute attribute)
        {
            if (attribute.LocalName != "Ignorable")
            {
                Add(XamlViolationCode.MarkupCompatibilityNotAllowed, $"Atributo '{attribute.QualifiedName}' não é permitido.", attribute.Line, attribute.Column);
                return;
            }

            foreach (string prefix in attribute.Value.Split(' ', StringSplitOptions.RemoveEmptyEntries))
            {
                string? ns = reader.LookupNamespace(prefix);
                if (ns is not (XamlSecurityPolicy.DesignerNamespace or XamlSecurityPolicy.PresentationOptionsNamespace))
                {
                    Add(XamlViolationCode.MarkupCompatibilityNotAllowed, $"Prefixo ignorável '{prefix}' não é permitido.", attribute.Line, attribute.Column);
                }
            }
        }

        private void VisitText()
        {
            // Ex.: <BitmapImage.UriSource>http://...</BitmapImage.UriSource>
            if (_elements.TryPeek(out string? parent) && parent.Contains('.', StringComparison.Ordinal))
            {
                ValidateValue(LastSegment(parent), reader.Value, null);
            }
        }

        private void ValidateValue(string property, string value, Attribute? attribute)
        {
            int line = attribute?.Line ?? _lineInfo.LineNumber;
            int column = attribute?.Column ?? _lineInfo.LinePosition;
            string trimmed = value.Trim();
            CheckPrefixReferences(trimmed, line, column);

            if (trimmed.StartsWith('{') && !trimmed.StartsWith("{}", StringComparison.Ordinal))
            {
                ValidateMarkupExtensions(trimmed, line, column);
                return;
            }

            if (IsAbsoluteLocation(trimmed))
            {
                if (!trimmed.StartsWith(AllowedPackPrefix, StringComparison.OrdinalIgnoreCase))
                {
                    Add(XamlViolationCode.ExternalUri, $"URI ou caminho absoluto '{trimmed}' não é permitido; use caminhos relativos ao tema.", line, column);
                }

                return;
            }

            if (policy.UriProperties.Contains(property) && EscapesTheme(trimmed))
            {
                Add(XamlViolationCode.PathEscapesTheme, $"Caminho '{trimmed}' sai da pasta do tema.", line, column);
            }
        }

        private void ValidateMarkupExtensions(string value, int line, int column)
        {
            foreach (Match match in MarkupExtensionPattern().Matches(value))
            {
                string prefix = match.Groups["prefix"].Value;
                string name = StripExtension(match.Groups["name"].Value);
                string argument = FirstArgument(match.Groups["args"].Value);
                string? ns = reader.LookupNamespace(prefix);

                switch (ns)
                {
                    case XamlSecurityPolicy.PresentationNamespace when policy.PresentationExtensions.Contains(name):
                        continue;
                    case XamlSecurityPolicy.XamlNamespace when name is "Null" or "Reference":
                        continue;
                    case XamlSecurityPolicy.XamlNamespace when name == "Static":
                        ValidateStaticMember(argument, line, column);
                        continue;
                    case XamlSecurityPolicy.XamlNamespace when name == "Type":
                        ValidateTypeReference(argument, line, column);
                        continue;
                }

                if (ns is not null && policy.ClrNamespaces.TryGetValue(ns, out var types)
                    && (types.Contains(name) || types.Contains(name + "Extension")))
                {
                    continue;
                }

                MarkUsed(ns);
                string shown = prefix.Length > 0 ? $"{prefix}:{name}" : name;
                Add(XamlViolationCode.MarkupExtensionNotAllowed, $"Extensão de marcação '{{{shown}}}' não é permitida.", line, column);
            }
        }

        private void ValidateStaticMember(string member, int line, int column)
        {
            int dot = member.LastIndexOf('.');
            if (dot <= 0)
            {
                Add(XamlViolationCode.StaticMemberNotAllowed, $"Membro estático '{member}' inválido.", line, column);
                return;
            }

            var (ns, type) = ResolveTypeName(member[..dot]);
            bool allowed = ns == XamlSecurityPolicy.PresentationNamespace
                ? policy.StaticMemberOwners.Contains(type)
                : ns is not null && policy.ClrNamespaces.TryGetValue(ns, out var types) && types.Contains(type);

            if (!allowed)
            {
                Add(XamlViolationCode.StaticMemberNotAllowed, $"Membro estático '{member}' não é permitido.", line, column);
            }
        }

        private void ValidateTypeReference(string typeName, int line, int column)
        {
            // Referenciar um tipo de apresentação só produz um objeto Type; o risco está em resolver
            // tipos de outros assemblies, o que os namespaces permitidos já limitam.
            var (ns, type) = ResolveTypeName(typeName);
            bool allowed = ns == XamlSecurityPolicy.PresentationNamespace
                || (ns is not null && policy.ClrNamespaces.TryGetValue(ns, out var types) && types.Contains(type));

            if (!allowed)
            {
                Add(XamlViolationCode.TypeNotAllowed, $"Referência ao tipo '{typeName}' não é permitida.", line, column);
            }
        }

        private (string? Namespace, string Type) ResolveTypeName(string qualified)
        {
            int colon = qualified.IndexOf(':', StringComparison.Ordinal);
            return colon < 0
                ? (reader.LookupNamespace(string.Empty), qualified)
                : (reader.LookupNamespace(qualified[..colon]), qualified[(colon + 1)..]);
        }

        private bool IsOwnerAllowed(string ns, string owner)
        {
            if (ns == XamlSecurityPolicy.PresentationNamespace)
            {
                return policy.PresentationTypes.Contains(owner) || policy.PresentationOwnerTypes.Contains(owner);
            }

            return policy.ClrNamespaces.TryGetValue(ns, out var types) && types.Contains(owner);
        }

        private bool EscapesTheme(string value)
        {
            string path = value.Split('#')[0].Trim();
            if (path.Length == 0)
            {
                return false;
            }

            if (path[0] is '/' or '\\')
            {
                return true;
            }

            var segments = new Stack<string>(themeRelativePath.Split(PathSeparators, StringSplitOptions.RemoveEmptyEntries).SkipLast(1));
            foreach (string segment in path.Split(PathSeparators, StringSplitOptions.RemoveEmptyEntries))
            {
                if (segment == "..")
                {
                    if (!segments.TryPop(out _))
                    {
                        return true;
                    }
                }
                else if (segment != ".")
                {
                    segments.Push(segment);
                }
            }

            return false;
        }

        private void Add(XamlViolationCode code, string message) =>
            Add(code, message, _lineInfo.LineNumber, _lineInfo.LinePosition);

        private void Add(XamlViolationCode code, string message, int line, int column) =>
            violations.Add(new(code, message, line, column));
    }

    private static bool IsAbsoluteLocation(string value) =>
        value.Contains("://", StringComparison.Ordinal)
        || value.StartsWith(@"\\", StringComparison.Ordinal)
        || DriveLetterPath().IsMatch(value);

    private static string FirstArgument(string args)
    {
        string first = args.Split(',')[0].Trim();
        int equals = first.IndexOf('=', StringComparison.Ordinal);
        return (equals >= 0 ? first[(equals + 1)..] : first).Trim();
    }

    private static string StripExtension(string name) =>
        name.EndsWith("Extension", StringComparison.Ordinal) ? name[..^"Extension".Length] : name;

    private static string LastSegment(string name) => name[(name.LastIndexOf('.') + 1)..];

    private sealed record Attribute(
        string Namespace,
        string Prefix,
        string LocalName,
        string QualifiedName,
        string Value,
        bool IsNamespaceDeclaration,
        int Line,
        int Column);
}
