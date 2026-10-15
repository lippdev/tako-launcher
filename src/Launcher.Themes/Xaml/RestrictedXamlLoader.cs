using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Markup;

namespace Launcher.Themes.Xaml;

/// <summary>Rejeição de um arquivo XAML de tema, com as violações encontradas na validação ou o erro do parser WPF.</summary>
public sealed class ThemeXamlRejectedException : Exception
{
    public ThemeXamlRejectedException()
    {
        Violations = [];
    }

    public ThemeXamlRejectedException(string message)
        : base(message)
    {
        Violations = [];
    }

    public ThemeXamlRejectedException(string message, Exception innerException)
        : base(message, innerException)
    {
        Violations = [];
    }

    public ThemeXamlRejectedException(string themeRelativePath, IReadOnlyList<XamlViolation> violations)
        : base($"XAML '{themeRelativePath}' rejeitado: {string.Join("; ", violations)}")
    {
        Violations = violations;
    }

    public IReadOnlyList<XamlViolation> Violations { get; }
}

/// <summary>
/// Carrega <see cref="ResourceDictionary"/> de temas só depois de validar o XAML com <see cref="XamlValidator"/>.
/// Deve ser chamado numa thread STA com Dispatcher (thread de UI).
/// </summary>
public sealed class RestrictedXamlLoader(XamlValidator validator)
{
    public XamlValidator Validator => validator;

    public ResourceDictionary LoadResourceDictionary(string themeRoot, string themeRelativePath)
    {
        ArgumentNullException.ThrowIfNull(themeRoot);
        ArgumentNullException.ThrowIfNull(themeRelativePath);

        string root = Path.GetFullPath(themeRoot);
        string fullPath = Path.GetFullPath(Path.Combine(root, themeRelativePath));
        if (!fullPath.StartsWith(Path.TrimEndingDirectorySeparator(root) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
        {
            throw new ThemeXamlRejectedException($"Caminho '{themeRelativePath}' sai da pasta do tema.");
        }

        // Lê uma vez: o mesmo conteúdo validado é o que o parser recebe.
        string xaml = File.ReadAllText(fullPath);
        return Parse(xaml, fullPath, themeRelativePath);
    }

    public ResourceDictionary Parse(string xaml, string fullPath, string themeRelativePath)
    {
        var result = validator.Validate(xaml, themeRelativePath, expectedRoot: "ResourceDictionary");
        if (!result.IsValid)
        {
            throw new ThemeXamlRejectedException(themeRelativePath, result.Violations);
        }

        var context = new ParserContext { BaseUri = new Uri(fullPath, UriKind.Absolute) };
        using var stream = new MemoryStream(Encoding.UTF8.GetBytes(Prepare(xaml, result)));
        try
        {
            var dictionary = (ResourceDictionary)XamlReader.Load(stream, context);
            Realize(dictionary);
            return dictionary;
        }
        catch (Exception ex) when (ex is XamlParseException or InvalidOperationException or NotSupportedException)
        {
            throw new ThemeXamlRejectedException($"XAML '{themeRelativePath}' inválido: {ex.Message}", ex);
        }
    }

    /// <summary>
    /// O WPF adia a criação de recursos e templates até o primeiro uso; um erro ali (ex.: propriedade que só existe no
    /// Playnite) derrubaria a UI depois. Materializa tudo agora, para o erro recusar o arquivo na carga (MVP-021).
    /// </summary>
    private static void Realize(ResourceDictionary dictionary)
    {
        foreach (object key in dictionary.Keys.Cast<object>().ToList())
        {
            switch (dictionary[key])
            {
                case Style style:
                    style.Seal();
                    foreach (var setter in style.Setters.OfType<Setter>())
                    {
                        if (setter.Value is FrameworkTemplate template)
                        {
                            _ = template.LoadContent();
                        }
                    }

                    break;
                case FrameworkTemplate template:
                    _ = template.LoadContent();
                    break;
            }
        }

        foreach (var merged in dictionary.MergedDictionaries)
        {
            Realize(merged);
        }
    }

    /// <summary>
    /// Ajustes depois da validação, sobre o mesmo texto validado: remove declarações de namespaces não permitidos
    /// que nada usa e aplica os remapeamentos da política (ex.: assembly do Playnite → adaptador P10).
    /// </summary>
    private string Prepare(string xaml, XamlValidationResult result)
    {
        foreach (var unused in result.UnusedDisallowedNamespaces)
        {
            xaml = Regex.Replace(xaml, $@"\s+xmlns:{Regex.Escape(unused.Prefix)}\s*=\s*([""']){Regex.Escape(unused.Namespace)}\1", string.Empty);
        }

        foreach (var (source, target) in validator.Policy.NamespaceRemaps)
        {
            xaml = Regex.Replace(xaml, $@"(xmlns:[\w.-]+\s*=\s*[""']){Regex.Escape(source)}([""'])", m => m.Groups[1].Value + target + m.Groups[2].Value);
        }

        return xaml;
    }
}
