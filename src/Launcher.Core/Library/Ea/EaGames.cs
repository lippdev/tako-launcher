using System.Xml;
using System.Xml.Linq;

namespace Launcher.Core.Library.Ea;

/// <summary>O que o <c>__Installer\installerdata.xml</c> de um jogo da EA diz: os ids de conteúdo e o nome.</summary>
public sealed record EaInstallerData(IReadOnlyList<string> ContentIds, string? Title);

/// <summary>Qual programa atende o protocolo da EA neste PC (STORE-012: ausente ou desatualizado).</summary>
public enum EaClientKind
{
    /// <summary>Nenhum: os jogos aparecem, mas não abrem.</summary>
    Missing,

    /// <summary>Só o Origin, desligado pela EA em 2025: o protocolo existe, mas não abre mais os jogos.</summary>
    LegacyOrigin,

    /// <summary>O EA app.</summary>
    EaApp,
}

/// <summary>
/// Formatos do EA app lidos sem o cliente aberto (ADR-0019): o <c>installerdata.xml</c> que acompanha cada jogo e o
/// comando registrado para os protocolos <c>origin2://</c> e <c>link2ea://</c>.
/// </summary>
public static class EaGames
{
    private static readonly string[] PreferredLocales = ["pt_BR", "en_US"];

    /// <summary>
    /// Lê o <c>installerdata.xml</c> (formatos "DiPManifest" do Origin e do EA app). Tolerante: aceita os ids em
    /// <c>contentIDs/contentID</c> em qualquer nível e o nome em <c>gameTitles/gameTitle</c> ou
    /// <c>localeInfo/title</c>, preferindo pt_BR e depois en_US. XML inválido lança <see cref="XmlException"/>.
    /// </summary>
    public static EaInstallerData Parse(string xml)
    {
        ArgumentNullException.ThrowIfNull(xml);
        var document = XDocument.Parse(xml);
        var ids = document.Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "contentID", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Value.Trim())
            .Where(v => v.Length > 0)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();

        // Nome por idioma: <gameTitle locale="pt_BR"> (EA app) ou <localeInfo locale="pt_BR"><title> (Origin).
        var titles = new List<(string? Locale, string Title)>();
        foreach (var element in document.Descendants())
        {
            string name = element.Name.LocalName;
            if (string.Equals(name, "gameTitle", StringComparison.OrdinalIgnoreCase))
            {
                titles.Add(((string?)element.Attribute("locale"), element.Value.Trim()));
            }
            else if (string.Equals(name, "title", StringComparison.OrdinalIgnoreCase)
                && element.Parent is { } parent && string.Equals(parent.Name.LocalName, "localeInfo", StringComparison.OrdinalIgnoreCase))
            {
                titles.Add(((string?)parent.Attribute("locale"), element.Value.Trim()));
            }
        }

        titles.RemoveAll(t => t.Title.Length == 0);
        string? title = PreferredLocales
            .Select(locale => titles.FirstOrDefault(t => string.Equals(t.Locale, locale, StringComparison.OrdinalIgnoreCase)).Title)
            .FirstOrDefault(t => t is not null)
            ?? titles.Select(t => t.Title).FirstOrDefault();
        return new EaInstallerData(ids, title);
    }

    /// <summary>
    /// Abrir pelo EA app: <c>origin2://game/launch?offerIds=&lt;ids&gt;&amp;autoDownload=1</c>. O EA app herdou o
    /// protocolo do Origin e aceita os ids de conteúdo do <c>installerdata.xml</c>; o <c>link2ea://launchgame/</c> que
    /// a Steam usa exige o id de oferta da loja, que não fica no disco. Ver ADR-0019 (incerteza documentada).
    /// </summary>
    public static string LaunchUri(IReadOnlyList<string> contentIds)
    {
        ArgumentNullException.ThrowIfNull(contentIds);
        return $"origin2://game/launch?offerIds={string.Join(',', contentIds.Select(Uri.EscapeDataString))}&autoDownload=1";
    }

    /// <summary>
    /// Quem atende o protocolo: o comando de <c>origin2</c> ou <c>link2ea</c> apontando para o EA app
    /// (<c>EADesktop.exe</c>, <c>EALauncher.exe</c>, <c>EALaunchHelper.exe</c>…), só para o <c>Origin.exe</c> ou nada.
    /// </summary>
    public static EaClientKind ClientKind(string? origin2Command, string? link2eaCommand)
    {
        if (!string.IsNullOrWhiteSpace(link2eaCommand) || IsEaApp(origin2Command))
        {
            return EaClientKind.EaApp;
        }

        return string.IsNullOrWhiteSpace(origin2Command) ? EaClientKind.Missing : EaClientKind.LegacyOrigin;
    }

    private static bool IsEaApp(string? command) =>
        command is not null
        && (command.Contains("EA Desktop", StringComparison.OrdinalIgnoreCase)
            || command.Contains("EADesktop", StringComparison.OrdinalIgnoreCase)
            || command.Contains("EALaunch", StringComparison.OrdinalIgnoreCase)
            || command.Contains("EALauncher", StringComparison.OrdinalIgnoreCase));
}
