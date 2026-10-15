namespace Launcher.Themes.Xaml;

public enum XamlViolationCode
{
    MalformedXml,
    DtdProhibited,
    LimitExceeded,
    UnexpectedRoot,
    NamespaceNotAllowed,
    TypeNotAllowed,
    DirectiveNotAllowed,
    MarkupExtensionNotAllowed,
    StaticMemberNotAllowed,
    MarkupCompatibilityNotAllowed,
    ExternalUri,
    PathEscapesTheme,
    NestedLoadNotAllowed,
}

/// <summary>Motivo pelo qual um arquivo XAML de tema foi rejeitado. Linha e coluna começam em 1 (0 quando desconhecidas).</summary>
public sealed record XamlViolation(XamlViolationCode Code, string Message, int Line, int Column)
{
    public override string ToString() => $"({Line},{Column}) {Code}: {Message}";
}

/// <summary>Declaração de namespace não permitido que nenhum elemento, atributo ou valor usa.</summary>
public sealed record UnusedNamespaceDeclaration(string Prefix, string Namespace);

public sealed class XamlValidationResult
{
    internal XamlValidationResult(IReadOnlyList<XamlViolation> violations, IReadOnlyList<UnusedNamespaceDeclaration>? unused = null)
    {
        Violations = violations;
        UnusedDisallowedNamespaces = unused ?? [];
    }

    public IReadOnlyList<XamlViolation> Violations { get; }

    /// <summary>
    /// Namespaces não permitidos apenas declarados (comum em temas Playnite, ex.: Interactivity). Não são violação;
    /// o carregador remove a declaração antes do parser para nunca resolver o assembly.
    /// </summary>
    public IReadOnlyList<UnusedNamespaceDeclaration> UnusedDisallowedNamespaces { get; }

    public bool IsValid => Violations.Count == 0;
}
