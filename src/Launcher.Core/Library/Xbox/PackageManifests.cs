using System.Xml;
using System.Xml.Linq;

namespace Launcher.Core.Library.Xbox;

/// <summary>Executável declarado no <c>MicrosoftGame.config</c>; o <see cref="Id"/> vira o id do app no AUMID.</summary>
public sealed record GameExecutable(string Name, string Id);

/// <summary>
/// <c>MicrosoftGame.config</c> de um jogo do GDK (PC Game Pass / app Xbox): identidade do pacote, executáveis e visual
/// da loja. Arquivo presente = o pacote é um jogo.
/// </summary>
public sealed record MicrosoftGameConfig
{
    public const string FileName = "MicrosoftGame.config";

    public required string IdentityName { get; init; }

    public required string Publisher { get; init; }

    public string? Version { get; init; }

    public IReadOnlyList<GameExecutable> Executables { get; init; } = [];

    /// <summary>Nome exibido (pode ser <c>ms-resource:</c>).</summary>
    public string? DisplayName { get; init; }

    public string? PublisherDisplayName { get; init; }

    public string? Square480x480Logo { get; init; }

    public string? Square150x150Logo { get; init; }

    public string? StoreLogo { get; init; }

    public string? SplashScreenImage { get; init; }

    public string? BackgroundColor { get; init; }

    public string? StoreId { get; init; }

    public string? TitleId { get; init; }

    public string FamilyName => PackageIdentity.FamilyName(IdentityName, Publisher);

    /// <summary>Lê o XML; nulo se não for um <c>MicrosoftGame.config</c> com identidade.</summary>
    public static MicrosoftGameConfig? Parse(string xml)
    {
        var root = PackageXml.Load(xml)?.Root;
        if (root is null || root.Name.LocalName != "Game" || PackageXml.Child(root, "Identity") is not { } identity
            || PackageXml.Attr(identity, "Name") is not { } name || PackageXml.Attr(identity, "Publisher") is not { } publisher)
        {
            return null;
        }

        var visuals = PackageXml.Child(root, "ShellVisuals");
        var executables = PackageXml.Child(root, "ExecutableList")?.Elements()
            .Where(e => e.Name.LocalName == "Executable")
            .Select(e => (Name: PackageXml.Attr(e, "Name"), Id: PackageXml.Attr(e, "Id")))
            .Where(e => e.Name is not null && e.Id is not null)
            .Select(e => new GameExecutable(e.Name!, e.Id!))
            .ToList() ?? [];
        return new MicrosoftGameConfig
        {
            IdentityName = name,
            Publisher = publisher,
            Version = PackageXml.Attr(identity, "Version"),
            Executables = executables,
            DisplayName = PackageXml.Attr(visuals, "DefaultDisplayName"),
            PublisherDisplayName = PackageXml.Attr(visuals, "PublisherDisplayName"),
            Square480x480Logo = PackageXml.Attr(visuals, "Square480x480Logo"),
            Square150x150Logo = PackageXml.Attr(visuals, "Square150x150Logo"),
            StoreLogo = PackageXml.Attr(visuals, "StoreLogo"),
            SplashScreenImage = PackageXml.Attr(visuals, "SplashScreenImage"),
            BackgroundColor = PackageXml.Attr(visuals, "BackgroundColor"),
            StoreId = PackageXml.Text(PackageXml.Child(root, "StoreId")),
            TitleId = PackageXml.Text(PackageXml.Child(root, "TitleId")),
        };
    }
}

/// <summary>App declarado no <c>AppxManifest.xml</c>.</summary>
public sealed record PackageApplication
{
    public required string Id { get; init; }

    public string? Executable { get; init; }

    public string? DisplayName { get; init; }

    public string? Square310x310Logo { get; init; }

    public string? Square150x150Logo { get; init; }

    public string? Wide310x150Logo { get; init; }

    public string? SplashScreen { get; init; }

    public string? BackgroundColor { get; init; }

    /// <summary><c>AppListEntry="none"</c>: o app não aparece no menu Iniciar (serviço, ajudante).</summary>
    public bool Hidden { get; init; }
}

/// <summary><c>AppxManifest.xml</c> de um pacote MSIX/AppX instalado (só o que o launcher usa).</summary>
public sealed record AppxManifest
{
    public const string FileName = "AppxManifest.xml";

    public required string IdentityName { get; init; }

    public required string Publisher { get; init; }

    public string? DisplayName { get; init; }

    public string? Logo { get; init; }

    /// <summary>Pacote de framework (ex.: VCLibs) ou de recursos: nunca é um jogo.</summary>
    public bool IsFrameworkOrResource { get; init; }

    public IReadOnlyList<PackageApplication> Applications { get; init; } = [];

    public string FamilyName => PackageIdentity.FamilyName(IdentityName, Publisher);

    /// <summary>App que o menu Iniciar abriria: o primeiro visível.</summary>
    public PackageApplication? MainApplication => Applications.FirstOrDefault(a => !a.Hidden);

    public static AppxManifest? Parse(string xml)
    {
        var root = PackageXml.Load(xml)?.Root;
        if (root is null || root.Name.LocalName != "Package" || PackageXml.Child(root, "Identity") is not { } identity
            || PackageXml.Attr(identity, "Name") is not { } name || PackageXml.Attr(identity, "Publisher") is not { } publisher)
        {
            return null;
        }

        var properties = PackageXml.Child(root, "Properties");
        bool framework = string.Equals(PackageXml.Text(PackageXml.Child(properties, "Framework")), "true", StringComparison.OrdinalIgnoreCase)
            || string.Equals(PackageXml.Text(PackageXml.Child(properties, "ResourcePackage")), "true", StringComparison.OrdinalIgnoreCase);
        var applications = PackageXml.Child(root, "Applications")?.Elements()
            .Where(e => e.Name.LocalName == "Application" && PackageXml.Attr(e, "Id") is not null)
            .Select(ToApplication)
            .ToList() ?? [];
        return new AppxManifest
        {
            IdentityName = name,
            Publisher = publisher,
            DisplayName = PackageXml.Text(PackageXml.Child(properties, "DisplayName")),
            Logo = PackageXml.Text(PackageXml.Child(properties, "Logo")),
            IsFrameworkOrResource = framework,
            Applications = applications,
        };
    }

    private static PackageApplication ToApplication(XElement app)
    {
        var visual = PackageXml.Child(app, "VisualElements");
        var tile = PackageXml.Child(visual, "DefaultTile");
        return new PackageApplication
        {
            Id = PackageXml.Attr(app, "Id")!,
            Executable = PackageXml.Attr(app, "Executable"),
            DisplayName = PackageXml.Attr(visual, "DisplayName"),
            Square150x150Logo = PackageXml.Attr(visual, "Square150x150Logo"),
            Square310x310Logo = PackageXml.Attr(tile, "Square310x310Logo"),
            Wide310x150Logo = PackageXml.Attr(tile, "Wide310x150Logo"),
            SplashScreen = PackageXml.Attr(PackageXml.Child(visual, "SplashScreen"), "Image"),
            BackgroundColor = PackageXml.Attr(visual, "BackgroundColor"),
            Hidden = string.Equals(PackageXml.Attr(visual, "AppListEntry"), "none", StringComparison.OrdinalIgnoreCase),
        };
    }
}

/// <summary>Leitura tolerante dos manifestos: ignora namespaces (uap, uap10…) e recusa DTD.</summary>
internal static class PackageXml
{
    private static readonly XmlReaderSettings Settings = new() { DtdProcessing = DtdProcessing.Prohibit, XmlResolver = null };

    public static XDocument? Load(string xml)
    {
        ArgumentNullException.ThrowIfNull(xml);
        try
        {
            using var text = new StringReader(xml);
            using var reader = XmlReader.Create(text, Settings);
            return XDocument.Load(reader);
        }
        catch (XmlException)
        {
            return null;
        }
    }

    public static XElement? Child(XElement? parent, string localName) =>
        parent?.Elements().FirstOrDefault(e => e.Name.LocalName == localName);

    public static string? Attr(XElement? element, string localName) =>
        element?.Attributes().FirstOrDefault(a => a.Name.LocalName == localName)?.Value is { } value && !string.IsNullOrWhiteSpace(value) ? value.Trim() : null;

    public static string? Text(XElement? element) =>
        element?.Value is { } value && !string.IsNullOrWhiteSpace(value) ? value.Trim() : null;
}
