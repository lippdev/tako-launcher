using System.Text.Json;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Display;
using Launcher.Core.Session.Devices;
using Launcher.Core.Session.Restore;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Session.GameScreen;

/// <summary>Marcador do modo console em disco: existe só enquanto o Tako está na tela de jogo por causa dele.</summary>
/// <param name="StartedAt">Quando o modo console começou.</param>
/// <param name="HdrDisplayId">Tela em que o Tako ligou o HDR (desligar numa recuperação); nula se não ligou.</param>
public sealed record ConsoleModeMarker(DateTimeOffset StartedAt, string? HdrDisplayId);

/// <summary>
/// <c>console-mode.json</c> na pasta de dados. Junto com o estado de monitores e áudio de antes (<c>console-mode-environment.json</c>,
/// ADR-0013), é o que permite voltar para a mesa se o Tako cair na tela de jogo. Falha de disco nunca impede nada.
/// </summary>
public sealed class ConsoleModeMarkerStore(string path)
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    public string FilePath { get; } = path;

    public bool Exists() => File.Exists(FilePath);

    public bool TryWrite(ConsoleModeMarker marker)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(FilePath))!); // caminho completo sempre tem pasta
            string temp = FilePath + ".tmp";
            File.WriteAllText(temp, JsonSerializer.Serialize(marker, Options));
            File.Move(temp, FilePath, overwrite: true);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>Marcador gravado; ausente devolve nulo, ilegível (gravação interrompida) devolve um marcador sem HDR.</summary>
    public ConsoleModeMarker? Read()
    {
        try
        {
            return File.Exists(FilePath) ? JsonSerializer.Deserialize<ConsoleModeMarker>(File.ReadAllText(FilePath), Options) ?? Unreadable : null;
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            return Unreadable;
        }
    }

    public void Delete()
    {
        try
        {
            File.Delete(FilePath);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Fica para a próxima abertura, que compara antes de mexer em qualquer coisa.
        }
    }

    private static ConsoleModeMarker Unreadable => new(DateTimeOffset.MinValue, null);
}

public enum ConsoleModeRecoveryOutcome
{
    /// <summary>A última abertura saiu do modo console direitinho: nada a fazer.</summary>
    NothingPending,

    /// <summary>O Tako caiu no modo console, mas as telas e o áudio já estavam como antes.</summary>
    AlreadyAtDesk,

    /// <summary>O Tako caiu no modo console e a mesa foi restaurada (e conferida).</summary>
    Restored,

    /// <summary>Restaurado em parte: alguma tela ou saída de áudio não voltou (detalhes no log).</summary>
    Incomplete,
}

/// <summary>
/// Recuperação do modo console (MVP-036): se o Tako caiu com a tela de jogo em foco, desliga o HDR que ele ligou e
/// devolve monitores e áudio ao estado de antes do modo console, <b>sem perguntar</b> (o usuário pode não estar vendo
/// tela nenhuma). Usada na abertura seguinte e pelo supervisor quando ele desiste de reabrir a UI (ADR-0012). Não
/// depende de WPF.
/// </summary>
public sealed partial class ConsoleModeRecovery(
    IDisplayControlService control,
    SessionEnvironmentGuard environment,
    PendingEnvironmentStore environmentStore,
    ConsoleModeMarkerStore markers,
    ILogger<ConsoleModeRecovery> logger)
{
    /// <summary>Há algo do modo console de uma execução anterior (marcador ou estado de antes em disco).</summary>
    public bool IsPending => markers.Exists() || environmentStore.Exists();

    public ConsoleModeRecoveryOutcome Recover()
    {
        if (!IsPending)
        {
            return ConsoleModeRecoveryOutcome.NothingPending;
        }

        var marker = markers.Read();
        LogRecovering(logger, marker?.StartedAt ?? DateTimeOffset.MinValue);

        // HDR primeiro: depois da restauração a tela de jogo pode nem estar mais ligada.
        if (marker?.HdrDisplayId is { } hdrId)
        {
            try
            {
                control.SetHdr(hdrId, enabled: false);
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogHdrFailed(logger, ex, hdrId);
            }
        }

        var outcome = ConsoleModeRecoveryOutcome.AlreadyAtDesk;
        if (environment.PendingFromPreviousRun() is { } pending)
        {
            outcome = environment.RestoreFrom(pending).Succeeded ? ConsoleModeRecoveryOutcome.Restored : ConsoleModeRecoveryOutcome.Incomplete;
        }

        markers.Delete();
        LogRecovered(logger, outcome);
        return outcome;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Modo console de uma execução anterior ({StartedAt}) não foi encerrado: voltando para a mesa")]
    private static partial void LogRecovering(ILogger logger, DateTimeOffset startedAt);

    [LoggerMessage(Level = LogLevel.Information, Message = "Recuperação do modo console: {Outcome}")]
    private static partial void LogRecovered(ILogger logger, ConsoleModeRecoveryOutcome outcome);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Recuperação do modo console: falha ao desligar o HDR de {Display}")]
    private static partial void LogHdrFailed(ILogger logger, Exception exception, string display);
}

/// <summary>Resultado de sair do modo console, para o aviso na tela.</summary>
public sealed record ConsoleModeExitReport(bool WasActive, bool ScreensRestored, RestoreReport Environment)
{
    public static ConsoleModeExitReport Nothing { get; } = new(false, true, RestoreReport.Nothing);

    /// <summary>Telas de volta como antes e nada que a verificação da ADR-0013 ainda ache diferente.</summary>
    public bool Succeeded => ScreensRestored && (Environment.NothingToDo || Environment.Succeeded);
}

/// <summary>
/// "Modo console" (MVP-002/012, ADR-0015): o Tako vai para a tela de jogo ao abrir, como o Console Mode, e só volta
/// para a mesa ao sair (ou em "Voltar ao PC"), não a cada jogo.
/// <list type="bullet">
/// <item>Entrar: registra monitores e áudio de antes (sempre, com a opção da ADR-0013 ligada ou não: é o caminho de
/// volta), grava o marcador, liga a TV (ADR-0016), foca a tela de jogo (<see cref="GameScreenSession"/>) e troca a
/// saída de áudio.</item>
/// <item>Sair: na ordem do fim de um jogo — tela de jogo, áudio e TV, restauração da ADR-0013 — e apaga o marcador.</item>
/// </list>
/// Lento (espera a TV e o Windows trocar as telas): chame fora da thread da UI. Entrar e sair nunca se cruzam.
/// </summary>
public sealed partial class ConsoleModeSession(
    GameScreenSession screen,
    GameSessionDevices devices,
    SessionEnvironmentGuard environment,
    ConsoleModeMarkerStore markers,
    ConsoleModeRecovery recovery,
    TimeProvider time,
    ILogger<ConsoleModeSession> logger) : IDisposable
{
    /// <summary>Contexto nos logs e nos registros de áudio/TV.</summary>
    public const string Context = "modo console";

    private readonly SemaphoreSlim _gate = new(1, 1);
    private volatile bool _active;

    /// <summary>A tela de jogo está em foco por causa do modo console (entre <see cref="EnterAsync"/> e <see cref="Exit"/>).</summary>
    public bool IsActive => _active;

    /// <summary>Uma execução anterior caiu no modo console (<see cref="RecoverFromPreviousRun"/> tem o que fazer).</summary>
    public bool HasPendingRecovery => !_active && recovery.IsPending;

    /// <summary>
    /// Vai para a tela de jogo. Só fica ativo com o resultado <see cref="GameScreenOutcome.Applied"/>; nos outros, nada
    /// ficou mudado. <see cref="GameScreenResult.NeedsConfirmation"/>: pergunte "Você consegue ver esta tela?" e, sem
    /// confirmação, chame <see cref="Exit"/>.
    /// </summary>
    public async Task<GameScreenResult> EnterAsync(GameScreenSettings settings, SessionDeviceOptions options, string? confirmedSetup, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(options);
        await _gate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_active)
            {
                return GameScreenResult.Skip("o modo console já está ativo");
            }

            if (screen.IsActive)
            {
                return GameScreenResult.Skip("a tela de jogo já está em foco");
            }

            var started = time.GetUtcNow();
            LogEntering(logger, options.Tv.IsActive);
            environment.BeginSession(Context, enabled: true); // sempre: é o caminho de volta se o Tako cair
            markers.TryWrite(new ConsoleModeMarker(started, null));

            bool tvReady = false;
            if (options.Tv.IsActive)
            {
                // A tela de jogo só aparece com a TV ligada na entrada do PC (ADR-0016).
                await devices.WakeTvAsync(Context, options.Tv).ConfigureAwait(false);
                tvReady = true;
            }

            var result = await Task.Run(() => screen.Begin(settings, confirmedSetup), CancellationToken.None).ConfigureAwait(false);
            if (result.Outcome != GameScreenOutcome.Applied)
            {
                environment.Discard();
                markers.Delete();
                LogNotEntered(logger, result.Outcome, result.Reason ?? "-");
                return result;
            }

            markers.TryWrite(new ConsoleModeMarker(started, screen.HdrDisplayId));
            _ = devices.BeginSession(Context, options, tvReady); // saída de áudio em segundo plano
            _active = true;
            LogEntered(logger, result.Target?.Name ?? settings.DisplayId);
            return result;
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>
    /// Volta para a mesa: tira a tela de jogo, devolve a saída de áudio (e põe a TV em espera, com
    /// <paramref name="standbyTv"/> e a opção ligada) e restaura monitores e áudio como estavam antes do modo console.
    /// Lento: chame fora da thread da UI. Sem modo console ativo, não faz nada.
    /// </summary>
    public ConsoleModeExitReport Exit(bool standbyTv)
    {
        _gate.Wait();
        try
        {
            if (!_active)
            {
                return ConsoleModeExitReport.Nothing;
            }

            _active = false;
            LogLeaving(logger, standbyTv);
            bool screens = screen.End();
            devices.EndSession(Context, steamGame: false, steamAppId: null, gameEnded: standbyTv);
            var report = environment.EndSession();
            markers.Delete();
            var exit = new ConsoleModeExitReport(true, screens, report);
            LogLeft(logger, exit.Succeeded);
            return exit;
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>Na abertura: o Tako caiu no modo console? Volta para a mesa sem perguntar (ver <see cref="ConsoleModeRecovery"/>).</summary>
    public ConsoleModeRecoveryOutcome RecoverFromPreviousRun()
    {
        _gate.Wait();
        try
        {
            return _active ? ConsoleModeRecoveryOutcome.NothingPending : recovery.Recover();
        }
        finally
        {
            _gate.Release();
        }
    }

    public void Dispose() => _gate.Dispose();

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console: indo para a tela de jogo (ligar a TV: {Tv})")]
    private static partial void LogEntering(ILogger logger, bool tv);

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console ativo em {Target}")]
    private static partial void LogEntered(ILogger logger, string target);

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console não ativado ({Outcome}): {Reason}")]
    private static partial void LogNotEntered(ILogger logger, GameScreenOutcome outcome, string reason);

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console: voltando para a mesa (TV em espera: {Standby})")]
    private static partial void LogLeaving(ILogger logger, bool standby);

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console encerrado; mesa como antes: {Succeeded}")]
    private static partial void LogLeft(ILogger logger, bool succeeded);
}
