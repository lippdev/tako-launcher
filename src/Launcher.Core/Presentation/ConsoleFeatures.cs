using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Tv;

namespace Launcher.Core.Presentation;

/// <summary>Estado de um recurso no menu Modo Console (ADR-0020): define a cor e o texto do selo do cartão.</summary>
public enum FeatureState
{
    Off,
    On,

    /// <summary>Ligado, mas falta um passo do usuário (escolher a tela, informar o IP, sair e entrar no Windows).</summary>
    NeedsSetup,

    /// <summary>Não funciona neste PC agora (tela desconectada, driver sem suporte, recurso fora desta versão).</summary>
    Problem,
}

/// <summary>Selo (estado) e linha de resumo de um cartão do Modo Console.</summary>
public sealed record FeatureStatus(FeatureState State, string Summary)
{
    public string Badge => ConsoleFeatures.Badge(State);
}

/// <summary>
/// Textos e decisões do menu Modo Console (ADR-0020), separados da tela para serem testados: o que cada cartão diz,
/// em linguagem de usuário, e a saída de áudio sugerida pelo assistente "Configurar para jogar na TV".
/// </summary>
public static class ConsoleFeatures
{
    public static string Badge(FeatureState state) => state switch
    {
        FeatureState.On => "Ligado",
        FeatureState.NeedsSetup => "Falta configurar",
        FeatureState.Problem => "Com problema",
        _ => "Desligado",
    };

    /// <param name="connected">Nulo quando não deu para ler as telas agora.</param>
    public static FeatureStatus GameScreen(bool enabled, string screenName, bool? connected) => (enabled, screenName.Length > 0, connected) switch
    {
        (false, _, _) => new(FeatureState.Off, "Os jogos abrem na tela em que o Tako está"),
        (true, false, _) => new(FeatureState.NeedsSetup, "Escolha a tela onde você joga"),
        (true, true, false) => new(FeatureState.Problem, $"{screenName} não está conectada; os jogos abrem na tela atual"),
        _ => new(FeatureState.On, $"Os jogos abrem em {screenName}"),
    };

    public static FeatureStatus Tv(TvControlConfig tv)
    {
        ArgumentNullException.ThrowIfNull(tv);
        if (!tv.Enabled)
        {
            return new(FeatureState.Off, "Liga a TV e troca para a entrada do PC ao abrir um jogo");
        }

        if (!tv.IsActive)
        {
            return new(FeatureState.NeedsSetup, "Informe o endereço IP da TV");
        }

        string standby = tv.StandbyAfterGame ? " · espera ao sair" : string.Empty;
        return new(FeatureState.On, $"{TvKinds.Label(tv.Kind)} · HDMI {tv.HdmiInput}{standby}");
    }

    /// <param name="outputName">Saída escolhida; nula = não alterar.</param>
    /// <param name="connected">Nulo quando não deu para ler as saídas agora.</param>
    public static FeatureStatus GameAudio(string? outputName, bool? connected) => (outputName, connected) switch
    {
        (null, _) => new(FeatureState.Off, "O som continua na saída que estiver em uso"),
        (_, false) => new(FeatureState.Problem, $"{outputName} está desconectada; o som fica na saída atual"),
        _ => new(FeatureState.On, $"Durante o jogo, o som sai por {outputName}"),
    };

    public static FeatureStatus SteamTray(bool on) => on
        ? new(FeatureState.On, "Ao sair de um jogo da Steam, a janela dela vai para a bandeja")
        : new(FeatureState.Off, "A janela da Steam fica como estiver ao sair do jogo");

    /// <param name="bundled">O medidor (PresentMon) veio nesta versão do Tako.</param>
    /// <param name="enabled">"Mostrar FPS" ligado.</param>
    public static FeatureStatus FpsCounter(bool bundled, bool enabled, PerfLogStatus permission, string styleLabel) => (bundled, enabled, permission) switch
    {
        (false, _, _) => new(FeatureState.Problem, "O contador de FPS não veio nesta versão do Tako"),
        (_, false, _) => new(FeatureState.Off, "Mede o FPS sem tocar no jogo; ligue com teclado e mouse"),
        (_, _, PerfLogStatus.NeedsSignOut) => new(FeatureState.NeedsSetup, "Saia e entre no Windows de novo para o contador funcionar"),
        (_, _, PerfLogStatus.NotGranted) => new(FeatureState.NeedsSetup, "Falta a permissão do Windows; desligue e ligue de novo"),
        _ => new(FeatureState.On, $"Estilo: {styleLabel}"),
    };

    /// <param name="capabilities">Nulo enquanto o driver ainda não foi consultado.</param>
    public static FeatureStatus FpsLimit(FrameRateLimitCapabilities? capabilities, int savedLimit, int? refreshHz) => capabilities switch
    {
        null => new(savedLimit == 0 ? FeatureState.Off : FeatureState.On, savedLimit == 0 ? "Sem limite · o driver é verificado no primeiro jogo" : $"{FpsLimits.Label(savedLimit, refreshHz)} · o driver é verificado no primeiro jogo"),
        { IsAvailable: false } caps => new(FeatureState.Problem, caps.Explanation),
        _ when savedLimit == 0 => new(FeatureState.Off, "Sem limite: o jogo roda no FPS que conseguir"),
        _ => new(FeatureState.On, $"Até {FpsLimits.Label(savedLimit, refreshHz)} em todos os jogos"),
    };

    /// <summary>
    /// Saída de áudio que combina com a tela de jogo (o Windows costuma dar ao áudio HDMI o nome da TV, ex.: "LG TV
    /// (NVIDIA High Definition Audio)" para a tela "LG TV"). Nula quando nenhuma combina.
    /// </summary>
    public static string? SuggestAudioOutput(string screenName, IEnumerable<(string Id, string Name)> outputs)
    {
        ArgumentNullException.ThrowIfNull(outputs);
        if (string.IsNullOrWhiteSpace(screenName))
        {
            return null;
        }

        string screen = screenName.Trim();
        var list = outputs.ToList();
        var exact = list.FirstOrDefault(o => o.Name.Contains(screen, StringComparison.OrdinalIgnoreCase));
        if (exact.Id is not null)
        {
            return exact.Id;
        }

        // Sem o nome inteiro: a primeira palavra marcante (a marca da TV) em comum.
        string? brand = screen.Split(' ', StringSplitOptions.RemoveEmptyEntries).FirstOrDefault(w => w.Length >= 2 && !IsGeneric(w));
        return brand is null ? null : list.FirstOrDefault(o => o.Name.Split(' ', '(', ')', '-').Any(w => string.Equals(w, brand, StringComparison.OrdinalIgnoreCase))).Id;
    }

    private static bool IsGeneric(string word) =>
        word.Equals("TV", StringComparison.OrdinalIgnoreCase) || word.Equals("Monitor", StringComparison.OrdinalIgnoreCase)
        || word.Equals("Tela", StringComparison.OrdinalIgnoreCase) || word.Equals("Display", StringComparison.OrdinalIgnoreCase)
        || word.Equals("Generic", StringComparison.OrdinalIgnoreCase) || word.Equals("Genérico", StringComparison.OrdinalIgnoreCase);
}
