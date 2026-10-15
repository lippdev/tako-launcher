using System.Globalization;

namespace Launcher.Core.Library.Xbox;

/// <summary>O que o Windows tem para os jogos do app Xbox funcionarem.</summary>
public enum XboxAvailability
{
    /// <summary>App Xbox e Serviços de Jogos presentes.</summary>
    Ready,

    /// <summary>Sem o app Xbox: o PC Game Pass não instala nem atualiza jogos.</summary>
    NoXboxApp,

    /// <summary>Sem os Serviços de Jogos (Gaming Services): jogos do GDK não abrem.</summary>
    NoGamingServices,

    /// <summary>Não foi possível ler os pacotes do usuário.</summary>
    Unknown,
}

/// <summary>Regras puras da fonte Xbox: o que é jogo, o que é app do sistema, cor e textos para o usuário.</summary>
public static class XboxGameRules
{
    /// <summary>Cor da marca Xbox; também o token <c>store-xbox</c> do DESIGN.md.</summary>
    public const string BrandColor = "#107C10";

    public const string XboxAppFamily = "Microsoft.GamingApp_8wekyb3d8bbwe";

    /// <summary>Arquivo de configuração do Xbox Live em jogos UWP antigos (antes do GDK).</summary>
    public const string XboxServicesConfig = "xboxservices.config";

    // Pacotes do próprio ecossistema Xbox/Store que não são jogos (alguns têm configuração do Xbox Live).
    private static readonly string[] SystemPackages =
    [
        "Microsoft.GamingApp", "Microsoft.XboxApp", "Microsoft.GamingServices", "Microsoft.XboxGamingOverlay", "Microsoft.XboxGameOverlay",
        "Microsoft.XboxIdentityProvider", "Microsoft.Xbox.TCUI", "Microsoft.XboxSpeechToTextOverlay", "Microsoft.WindowsStore",
        "Microsoft.StorePurchaseApp", "Microsoft.XboxGameCallableUI", "Microsoft.XboxDevices",
    ];

    public static bool IsSystemPackage(string familyOrName)
    {
        string name = PackageIdentity.NameOf(familyOrName);
        return SystemPackages.Contains(name, StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Um pacote é jogo quando tem <c>MicrosoftGame.config</c> (GDK), tem <c>xboxservices.config</c> (UWP com Xbox Live)
    /// ou os Serviços de Jogos o registram; nunca framework, recursos ou apps do próprio Xbox.
    /// </summary>
    public static bool IsGame(string family, bool hasGameConfig, bool hasXboxServicesConfig, bool knownToGamingServices, AppxManifest? manifest)
    {
        ArgumentNullException.ThrowIfNull(family);
        if (IsSystemPackage(family) || manifest is { IsFrameworkOrResource: true } || (manifest is { MainApplication: null } && !hasGameConfig))
        {
            return false;
        }

        return hasGameConfig || hasXboxServicesConfig || knownToGamingServices;
    }

    /// <summary>
    /// Família de cada entrada da lista dos Serviços de Jogos, que traz nomes completos ou famílias.
    /// </summary>
    public static HashSet<string> Families(IEnumerable<string> fullOrFamilyNames)
    {
        ArgumentNullException.ThrowIfNull(fullOrFamilyNames);
        var families = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (string entry in fullOrFamilyNames)
        {
            if (PackageIdentity.TryFamilyFromFullName(entry, out string family))
            {
                families.Add(family);
            }
            else if (entry.Split('_') is [{ Length: > 0 }, { Length: 13 }])
            {
                families.Add(entry);
            }
        }

        return families;
    }

    /// <summary>Cor de fundo do manifesto (#RRGGBB) como destaque; "transparent" ou inválida → cor do Xbox.</summary>
    public static string Accent(params string?[] colors)
    {
        ArgumentNullException.ThrowIfNull(colors);
        foreach (string? color in colors)
        {
            if (color is { Length: 7 } && color[0] == '#' && int.TryParse(color.AsSpan(1), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out _))
            {
                return color.ToUpperInvariant();
            }
        }

        return BrandColor;
    }

    /// <summary>Nome legível quando o manifesto não dá um: pasta do jogo ou o fim do nome do pacote.</summary>
    public static string FallbackTitle(string family, string? gameFolderName)
    {
        ArgumentNullException.ThrowIfNull(family);
        if (!string.IsNullOrWhiteSpace(gameFolderName) && !string.Equals(gameFolderName, "Content", StringComparison.OrdinalIgnoreCase))
        {
            return gameFolderName.Trim();
        }

        string name = PackageIdentity.NameOf(family);
        int dot = name.LastIndexOf('.');
        return dot >= 0 && dot < name.Length - 1 ? name[(dot + 1)..] : name;
    }

    public static XboxAvailability Availability(IReadOnlyCollection<string>? userFamilies, bool gamingServicesInstalled)
    {
        if (userFamilies is null)
        {
            return XboxAvailability.Unknown;
        }

        if (!userFamilies.Contains(XboxAppFamily, StringComparer.OrdinalIgnoreCase))
        {
            return XboxAvailability.NoXboxApp;
        }

        return gamingServicesInstalled ? XboxAvailability.Ready : XboxAvailability.NoGamingServices;
    }

    /// <summary>Linha de ajuda nas Configurações.</summary>
    public static string Describe(bool enabled, XboxAvailability availability, int games)
    {
        if (!enabled)
        {
            return "Desligado: jogos do app Xbox e da Microsoft Store não aparecem na biblioteca";
        }

        string found = games switch
        {
            0 => "nenhum jogo encontrado",
            1 => "1 jogo encontrado",
            _ => $"{games} jogos encontrados",
        };
        return availability switch
        {
            XboxAvailability.NoXboxApp => $"App Xbox não encontrado: instale-o pela Microsoft Store para jogar o PC Game Pass ({found})",
            XboxAvailability.NoGamingServices => $"Serviços de Jogos não encontrados: abra o app Xbox para instalá-los ({found})",
            XboxAvailability.Unknown => $"Não foi possível ler os apps instalados deste usuário ({found})",
            _ => $"PC Game Pass e Microsoft Store · {found}",
        };
    }
}
