// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ConsoleEngine.cs (Start/Stop), src/ConsoleMode/Services/MonitorService.cs (EnsureActive) e src/ConsoleMode/Services/VideoFeaturesService.cs (HDR)
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Display;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Session.GameScreen;

public enum GameScreenOutcome
{
    /// <summary>Nada foi alterado (ex.: a tela escolhida não está conectada); o jogo abre como está.</summary>
    Skipped,

    /// <summary>A tela de jogo está em foco; <see cref="GameScreenSession.End"/> desfaz.</summary>
    Applied,

    /// <summary>Algo deu errado no meio; as telas já voltaram como estavam.</summary>
    Reverted,
}

/// <summary>Resultado de <see cref="GameScreenSession.Begin"/>.</summary>
public sealed record GameScreenResult(GameScreenOutcome Outcome, string? Reason, ConnectedDisplay? Target, ScreenBounds? TargetBounds, bool NeedsConfirmation)
{
    public static GameScreenResult Skip(string reason) => new(GameScreenOutcome.Skipped, reason, null, null, false);
}

/// <summary>
/// "Tela de jogo" (MVP-002, como o Console Mode): ao abrir o jogo, liga a tela escolhida (ex.: a TV), torna-a principal,
/// esconde as outras (desconecta ou cobre de preto), aplica resolução/taxa e liga o HDR; ao terminar, desfaz tudo.
/// Antes de mexer, lê a topologia pelo <see cref="IDisplayConfigurationService"/> (ADR-0013) e a restauração devolve
/// exatamente essa topologia. Nunca derruba a sessão: falhas viram <see cref="GameScreenOutcome.Reverted"/> e log.
/// </summary>
public sealed partial class GameScreenSession
{
    /// <summary>Quanto esperar a tela ligar (uma TV saindo do modo de espera leva alguns segundos).</summary>
    public static readonly TimeSpan WakeTimeout = TimeSpan.FromSeconds(8);

    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(400);

    private readonly IDisplayControlService _control;
    private readonly IDisplayConfigurationService _configuration;
    private readonly IScreenCurtain _curtain;
    private readonly Action<TimeSpan> _sleep;
    private readonly ILogger<GameScreenSession> _logger;
    private readonly object _gate = new();
    private Active? _active;

    public GameScreenSession(
        IDisplayControlService control,
        IDisplayConfigurationService configuration,
        IScreenCurtain curtain,
        ILogger<GameScreenSession> logger,
        Action<TimeSpan>? sleep = null)
    {
        _control = control;
        _configuration = configuration;
        _curtain = curtain;
        _logger = logger;
        _sleep = sleep ?? Thread.Sleep;
    }

    /// <summary>A tela de jogo está em foco (entre <see cref="Begin"/> e <see cref="End"/>).</summary>
    public bool IsActive
    {
        get
        {
            lock (_gate)
            {
                return _active is not null;
            }
        }
    }

    /// <summary>Tela em que a sessão ligou o HDR (para desligar numa recuperação depois de uma queda); nula se não ligou.</summary>
    public string? HdrDisplayId
    {
        get
        {
            lock (_gate)
            {
                return _active?.HdrTurnedOnFor;
            }
        }
    }

    /// <summary>Telas conectadas para escolher em Configurações; falha da plataforma devolve lista vazia.</summary>
    public IReadOnlyList<ConnectedDisplay> ListDisplays()
    {
        try
        {
            return _control.ListDisplays();
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogListFailed(_logger, ex);
            return [];
        }
    }

    /// <summary>Modos da tela para escolher em Configurações (vazio se estiver desligada).</summary>
    public IReadOnlyList<DisplayMode> ListModes(string displayId)
    {
        try
        {
            return _control.ListModes(displayId);
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogListFailed(_logger, ex);
            return [];
        }
    }

    /// <summary>
    /// Foca a tela de jogo. Lento (espera a tela ligar): chame fora da thread da UI. <paramref name="confirmedSetup"/>
    /// é a última combinação confirmada; diferente dela, o resultado pede confirmação.
    /// </summary>
    public GameScreenResult Begin(GameScreenSettings settings, string? confirmedSetup)
    {
        ArgumentNullException.ThrowIfNull(settings);
        lock (_gate)
        {
            if (_active is not null)
            {
                return GameScreenResult.Skip("a tela de jogo já está em foco");
            }

            GameScreenPlan plan;
            DisplayTopology before;
            try
            {
                plan = GameScreenPlanner.Plan(settings, _control.ListDisplays());
                if (!plan.CanApply)
                {
                    LogSkipped(_logger, plan.Problem ?? "-");
                    return GameScreenResult.Skip(plan.Problem ?? "nada a fazer");
                }

                // Sem a foto de antes não há como voltar: melhor não mexer.
                before = _configuration.Capture();
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogBeginFailed(_logger, ex, settings.DisplayId);
                return GameScreenResult.Skip("não foi possível ler as telas");
            }

            var target = plan.Target!;
            _active = new Active(before, settings.HideOthers);
            LogBegin(_logger, target.Name, settings.HideOthers, plan.HideIds.Count, plan.Mode, plan.TurnOnHdr);
            foreach (string note in plan.Notes)
            {
                LogNote(_logger, note);
            }

            try
            {
                return Apply(settings, plan, target, confirmedSetup);
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogBeginFailed(_logger, ex, target.Name);
                RestoreLocked();
                return new GameScreenResult(GameScreenOutcome.Reverted, ex.Message, target, null, false);
            }
        }
    }

    /// <summary>Desfaz a tela de jogo: tira a cortina, desliga o HDR que foi ligado e devolve a topologia de antes.</summary>
    /// <returns>Falso se a verificação depois de restaurar ainda encontrou diferença (detalhes no log).</returns>
    public bool End()
    {
        lock (_gate)
        {
            return _active is null || RestoreLocked();
        }
    }

    private GameScreenResult Apply(GameScreenSettings settings, GameScreenPlan plan, ConnectedDisplay target, string? confirmedSetup)
    {
        var active = _active!;
        string id = target.Id;
        if (plan.EnableTarget)
        {
            _control.Enable([id]);
            // Nunca esconder as outras telas enquanto a de jogo ainda está apagada.
            if (!WaitActive(id))
            {
                _control.Enable([id]);
                if (!WaitActive(id))
                {
                    LogDidNotWake(_logger, target.Name);
                    RestoreLocked();
                    return new GameScreenResult(GameScreenOutcome.Reverted, $"{target.Name} não ligou", target, null, false);
                }
            }
        }

        _control.SetPrimary(id);
        if (plan.HideIds.Count > 0)
        {
            if (settings.HideOthers == HideOthersMode.Disconnect)
            {
                _control.Disable(plan.HideIds);
                // Desconectar pode mexer nas posições: a de jogo continua principal.
                _control.SetPrimary(id);
            }
            else
            {
                var covered = _control.ListDisplays()
                    .Where(d => d.IsActive && d.Bounds is not null && plan.HideIds.Contains(d.Id, StringComparer.OrdinalIgnoreCase))
                    .Select(d => d.Bounds!.Value)
                    .ToList();
                _curtain.Show(covered);
                active.CurtainShown = covered.Count > 0;
            }
        }

        if (plan.Mode is { } mode)
        {
            try
            {
                _control.SetMode(id, mode);
            }
            catch (RuntimeEnvironmentException ex)
            {
                // Resolução recusada não impede o jogo: segue no modo atual da tela.
                LogModeFailed(_logger, ex, mode.ToString(), target.Name);
            }
        }

        if (plan.TurnOnHdr)
        {
            var now = Find(id);
            if (now?.Hdr == HdrStatus.Off)
            {
                try
                {
                    _control.SetHdr(id, enabled: true);
                    active.HdrTurnedOnFor = id;
                }
                catch (RuntimeEnvironmentException ex)
                {
                    LogHdrFailed(_logger, ex, target.Name);
                }
            }
            else if (now?.Hdr != HdrStatus.On)
            {
                LogHdrUnavailable(_logger, target.Name);
            }
        }

        var bounds = Find(id)?.Bounds;
        bool confirm = settings.NeedsConfirmation(confirmedSetup);
        LogApplied(_logger, target.Name, bounds, confirm);
        return new GameScreenResult(GameScreenOutcome.Applied, null, target, bounds, confirm);
    }

    private bool RestoreLocked()
    {
        var active = _active!;
        _active = null;
        if (active.CurtainShown || active.HideOthers == HideOthersMode.BlackCurtain)
        {
            _curtain.Close();
        }

        if (active.HdrTurnedOnFor is { } hdrId)
        {
            try
            {
                _control.SetHdr(hdrId, enabled: false);
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogHdrFailed(_logger, ex, hdrId);
            }
        }

        // Duas tentativas: depois de religar telas o Windows às vezes reposiciona tudo de novo (Console Mode, RestoreBackup).
        for (int attempt = 1; attempt <= 2; attempt++)
        {
            try
            {
                var now = _configuration.Capture();
                if (now.IsEquivalentTo(active.Before))
                {
                    LogRestored(_logger, attempt - 1);
                    return true;
                }

                var changed = active.Before.Monitors.Select(m => m.Id).Union(now.Monitors.Select(m => m.Id), StringComparer.OrdinalIgnoreCase).ToList();
                _configuration.Apply(active.Before, changed);
                _sleep(PollInterval);
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogRestoreFailed(_logger, ex, attempt);
            }
        }

        try
        {
            if (_configuration.Capture().IsEquivalentTo(active.Before))
            {
                LogRestored(_logger, 2);
                return true;
            }
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogRestoreFailed(_logger, ex, 3);
        }

        LogRestoreIncomplete(_logger);
        return false;
    }

    private bool WaitActive(string id)
    {
        var waited = TimeSpan.Zero;
        while (true)
        {
            if (Find(id) is { IsActive: true })
            {
                return true;
            }

            if (waited >= WakeTimeout)
            {
                return false;
            }

            _sleep(PollInterval);
            waited += PollInterval;
        }
    }

    private ConnectedDisplay? Find(string id) =>
        _control.ListDisplays().FirstOrDefault(d => string.Equals(d.Id, id, StringComparison.OrdinalIgnoreCase));

    private sealed class Active(DisplayTopology before, HideOthersMode hideOthers)
    {
        public DisplayTopology Before { get; } = before;

        public HideOthersMode HideOthers { get; } = hideOthers;

        public bool CurtainShown { get; set; }

        public string? HdrTurnedOnFor { get; set; }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo: {Target}, outras telas: {HideOthers} ({Hidden}), modo: {Mode}, ligar HDR: {Hdr}")]
    private static partial void LogBegin(ILogger logger, string target, HideOthersMode hideOthers, int hidden, DisplayMode? mode, bool hdr);

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo em foco: {Target} {Bounds}; pedir confirmação: {Confirm}")]
    private static partial void LogApplied(ILogger logger, string target, ScreenBounds? bounds, bool confirm);

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo não aplicada: {Reason}")]
    private static partial void LogSkipped(ILogger logger, string reason);

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo: {Note}")]
    private static partial void LogNote(ILogger logger, string note);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: falha ao focar {Target}; voltando as telas")]
    private static partial void LogBeginFailed(ILogger logger, Exception exception, string target);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: {Target} não ligou a tempo; voltando as telas")]
    private static partial void LogDidNotWake(ILogger logger, string target);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: {Mode} recusado em {Target}; mantendo o modo atual")]
    private static partial void LogModeFailed(ILogger logger, Exception exception, string mode, string target);

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo: HDR não disponível em {Target}")]
    private static partial void LogHdrUnavailable(ILogger logger, string target);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: falha ao mudar o HDR de {Target}")]
    private static partial void LogHdrFailed(ILogger logger, Exception exception, string target);

    [LoggerMessage(Level = LogLevel.Information, Message = "Tela de jogo desfeita: telas como antes ({Attempts} restauração(ões))")]
    private static partial void LogRestored(ILogger logger, int attempts);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: falha ao restaurar as telas (tentativa {Attempt})")]
    private static partial void LogRestoreFailed(ILogger logger, Exception exception, int attempt);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tela de jogo: as telas não voltaram exatamente como antes")]
    private static partial void LogRestoreIncomplete(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível listar as telas")]
    private static partial void LogListFailed(ILogger logger, Exception exception);
}
