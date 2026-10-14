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

public sealed class XamlValidationResult
{
    internal XamlValidationResult(IReadOnlyList<XamlViolation> violations) => Violations = violations;

    public IReadOnlyList<XamlViolation> Violations { get; }

    public bool IsValid => Violations.Count == 0;
}
