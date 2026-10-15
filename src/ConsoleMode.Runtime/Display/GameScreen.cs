// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ConsoleEngine.cs (Start) e src/ConsoleMode/Models/Models.cs (SetupKey)
namespace ConsoleMode.Runtime.Display;

/// <summary>O que fazer com as outras telas enquanto se joga na tela de jogo.</summary>
public enum HideOthersMode
{
    /// <summary>Desconecta da área de trabalho (como Win+P "Somente segunda tela"): janelas e mouse não vão para lá.</summary>
    Disconnect,

    /// <summary>Cobre de preto: continuam ligadas, mas sem imagem (para drivers que reorganizam tudo ao desconectar).</summary>
    BlackCurtain,
}

/// <summary>Preferências da "Tela de jogo" (Configurações).</summary>
public sealed record GameScreenSettings(string DisplayId, HideOthersMode HideOthers, DisplayMode? Mode, bool EnableHdr)
{
    /// <summary>
    /// Tela + modo que o usuário confirmou estar vendo (<c>id|LxA@Hz</c> ou <c>id|current</c>). Uma combinação diferente
    /// pergunta "Você consegue ver esta tela?" antes de abrir o jogo e volta sozinha se ninguém responder.
    /// </summary>
    public string SetupKey => $"{DisplayId}|{Mode?.Key ?? "current"}";

    public bool NeedsConfirmation(string? confirmedSetup) => !string.Equals(SetupKey, confirmedSetup, StringComparison.OrdinalIgnoreCase);
}

/// <summary>
/// Passos para focar a tela de jogo, na ordem do Console Mode: ligar a tela (se estiver desligada) e esperar ela
/// aparecer, torná-la principal, esconder as outras, aplicar o modo e ligar o HDR. <see cref="Problem"/> preenchido:
/// nada a fazer (o jogo abre como está).
/// </summary>
public sealed record GameScreenPlan(
    ConnectedDisplay? Target,
    bool EnableTarget,
    IReadOnlyList<string> HideIds,
    DisplayMode? Mode,
    bool TurnOnHdr,
    IReadOnlyList<string> Notes,
    string? Problem)
{
    public bool CanApply => Problem is null && Target is not null;
}

public static class GameScreenPlanner
{
    public static GameScreenPlan Plan(GameScreenSettings settings, IReadOnlyList<ConnectedDisplay> displays)
    {
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(displays);
        var target = displays.FirstOrDefault(d => string.Equals(d.Id, settings.DisplayId, StringComparison.OrdinalIgnoreCase));
        if (target is null)
        {
            return new GameScreenPlan(null, false, [], null, false, [], "a tela de jogo escolhida não está conectada");
        }

        var notes = new List<string>();
        var hide = displays
            .Where(d => d.IsActive && !ReferenceEquals(d, target))
            .Select(d => d.Id)
            .ToList();

        // Modo pedido: só se for diferente do atual (com a tela desligada não dá para saber, então aplica).
        var mode = settings.Mode is { IsValid: true } wanted && (!target.IsActive || target.CurrentMode != wanted) ? wanted : null;

        bool turnOnHdr = false;
        if (settings.EnableHdr)
        {
            switch (target.Hdr)
            {
                case HdrStatus.Unsupported:
                    notes.Add($"HDR não disponível em {target.Name}");
                    break;
                case HdrStatus.On:
                    break;
                default:
                    // Desligado, ou desconhecido porque a tela ainda está desligada: tenta depois de ligá-la.
                    turnOnHdr = true;
                    break;
            }
        }

        return new GameScreenPlan(target, !target.IsActive, hide, mode, turnOnHdr, notes, null);
    }
}
