using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Session.Restore;

/// <summary>
/// Snapshot pendente em disco (<c>session-environment.json</c> na pasta de dados): gravado antes do jogo e apagado
/// depois da restauração. Se o Tako cair no meio, o arquivo continua lá para a próxima abertura. Falhas de disco nunca
/// impedem o jogo de abrir.
/// </summary>
public sealed class PendingEnvironmentStore(string path)
{
    public string FilePath { get; } = path;

    public bool TryWrite(SessionEnvironmentSnapshot snapshot)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(FilePath))!); // caminho completo sempre tem pasta
            string temp = FilePath + ".tmp";
            File.WriteAllText(temp, SessionEnvironmentSnapshotJson.Serialize(snapshot));
            File.Move(temp, FilePath, overwrite: true);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>Snapshot pendente; arquivo ausente, ilegível ou de versão desconhecida devolve nulo.</summary>
    public SessionEnvironmentSnapshot? Read()
    {
        try
        {
            return File.Exists(FilePath) ? SessionEnvironmentSnapshotJson.TryDeserialize(File.ReadAllText(FilePath)) : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    public bool Exists() => File.Exists(FilePath);

    public void Delete()
    {
        try
        {
            File.Delete(FilePath);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Fica para a próxima abertura, que compara antes de oferecer qualquer coisa.
        }
    }
}

/// <summary>Resultado de uma restauração, com a verificação feita logo depois.</summary>
public sealed record RestoreReport(
    RestorePlan Plan,
    IReadOnlyList<RestoreStep> Applied,
    IReadOnlyList<SkippedRestore> Failed,
    EnvironmentDiff Remaining)
{
    public static RestoreReport Nothing { get; } = new(RestorePlan.Empty, [], [], EnvironmentDiff.None);

    /// <summary>Nada diferente do registrado (ou nada que dê para corrigir).</summary>
    public bool NothingToDo => Plan.IsEmpty;

    /// <summary>Tudo que foi planejado foi aplicado e a verificação confirmou.</summary>
    public bool Succeeded => Failed.Count == 0 && Remaining.IsEmpty;
}

/// <summary>
/// MVP-012: registra monitores e áudio antes do jogo e restaura depois só o que mudou, com verificação e log. As
/// chamadas ao Windows ficam atrás de <see cref="IDisplayConfigurationService"/> e <see cref="IAudioEndpointService"/>;
/// falha numa parte não impede a outra e nunca derruba a sessão.
/// </summary>
public sealed partial class SessionEnvironmentGuard(
    IDisplayConfigurationService display,
    IAudioEndpointService audio,
    PendingEnvironmentStore store,
    TimeProvider time,
    ILogger<SessionEnvironmentGuard> logger)
{
    private SessionEnvironmentSnapshot? _active;

    /// <summary>Snapshot da sessão em andamento (nulo fora de um jogo ou com a opção desligada).</summary>
    public SessionEnvironmentSnapshot? Active => _active;

    /// <summary>
    /// Antes de abrir o jogo: lê monitores e áudio e grava em disco. Com <paramref name="enabled"/> falso (opção
    /// desligada), não registra nada. Devolve nulo se nenhuma das partes pôde ser lida.
    /// </summary>
    public SessionEnvironmentSnapshot? BeginSession(string context, bool enabled)
    {
        _active = null;
        if (!enabled)
        {
            store.Delete();
            return null;
        }

        var snapshot = Capture(context);
        if (snapshot.Display is null && snapshot.Audio is null)
        {
            store.Delete();
            return null;
        }

        _active = snapshot;
        if (!store.TryWrite(snapshot))
        {
            LogPersistFailed(logger, store.FilePath);
        }

        LogCaptured(logger, context, snapshot.Display?.Monitors.Count ?? 0, snapshot.Audio?.Devices.Count ?? 0);
        return snapshot;
    }

    /// <summary>Fim do jogo (estado Restoring): compara com o atual e restaura só o que mudou.</summary>
    public RestoreReport EndSession()
    {
        var snapshot = _active;
        _active = null;
        return snapshot is null ? RestoreReport.Nothing : RestoreFrom(snapshot);
    }

    /// <summary>A sessão terminou sem jogo para restaurar (não abriu, não deu para acompanhar): esquece o registro.</summary>
    public void Discard()
    {
        _active = null;
        store.Delete();
    }

    /// <summary>
    /// Na abertura: snapshot deixado por uma sessão que não terminou (o Tako caiu durante o jogo) e que ainda difere
    /// do estado atual. Sem diferença que dê para corrigir, o arquivo é apagado e nada é oferecido.
    /// </summary>
    public SessionEnvironmentSnapshot? PendingFromPreviousRun()
    {
        if (!store.Exists())
        {
            return null;
        }

        var pending = store.Read();
        if (pending is null)
        {
            LogPendingUnreadable(logger, store.FilePath);
            store.Delete();
            return null;
        }

        var plan = RestorePlanner.Plan(pending, Capture(pending.Context), audio.CanSetDefaultDevice);
        if (plan.IsEmpty)
        {
            store.Delete();
            return null;
        }

        LogPendingFound(logger, pending.Context ?? "-", pending.CapturedAt, plan.Steps.Count);
        return pending;
    }

    /// <summary>Restaura <paramref name="expected"/>: diferença, plano, execução passo a passo e verificação.</summary>
    public RestoreReport RestoreFrom(SessionEnvironmentSnapshot expected)
    {
        ArgumentNullException.ThrowIfNull(expected);
        string context = expected.Context ?? "-";
        var plan = RestorePlanner.Plan(expected, Capture(expected.Context), audio.CanSetDefaultDevice);
        foreach (var skipped in plan.Skipped)
        {
            LogStepSkipped(logger, context, skipped.What, skipped.Reason);
        }

        if (plan.IsEmpty)
        {
            store.Delete();
            LogNothingToRestore(logger, context);
            return new RestoreReport(plan, [], [], EnvironmentDiff.None);
        }

        var applied = new List<RestoreStep>();
        var failed = new List<SkippedRestore>();
        foreach (var step in plan.Steps)
        {
            try
            {
                Execute(step);
                applied.Add(step);
                LogStepApplied(logger, context, step.Description);
            }
            catch (RuntimeEnvironmentException ex)
            {
                failed.Add(new SkippedRestore(step.Description, ex.Message));
                LogStepFailed(logger, ex, context, step.Description);
            }
        }

        // Verificação: lê de novo e compara só o que o plano tentou corrigir.
        var remaining = EnvironmentDiff.Between(expected, Capture(expected.Context)).RestrictTo(plan);
        var report = new RestoreReport(plan, applied, failed, remaining);
        if (report.Succeeded)
        {
            LogRestoreVerified(logger, context, applied.Count);
        }
        else
        {
            LogRestoreIncomplete(logger, context, applied.Count, plan.Steps.Count, remaining.ToString());
        }

        store.Delete();
        return report;
    }

    private SessionEnvironmentSnapshot Capture(string? context)
    {
        DisplayTopology? topology = null;
        AudioState? sound = null;
        try
        {
            topology = display.Capture();
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogCaptureFailed(logger, ex, "monitores");
        }

        try
        {
            sound = audio.Capture();
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogCaptureFailed(logger, ex, "áudio");
        }

        return SessionEnvironmentSnapshot.Create(time.GetUtcNow(), context, topology, sound);
    }

    private void Execute(RestoreStep step)
    {
        switch (step)
        {
            case RestoreDisplayStep d:
                display.Apply(d.Target, d.ChangedMonitorIds);
                break;
            case SetDefaultAudioStep a:
                audio.SetDefaultDevice(a.DeviceId, a.Role);
                break;
            case SetVolumeStep v:
                audio.SetVolume(v.DeviceId, v.Volume);
                break;
            case SetMuteStep m:
                audio.SetMute(m.DeviceId, m.Muted);
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(step), step, "Passo de restauração desconhecido");
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Monitores e áudio registrados antes de {Context}: {Monitors} monitor(es), {AudioDevices} saída(s) de áudio")]
    private static partial void LogCaptured(ILogger logger, string context, int monitors, int audioDevices);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível ler {Part}")]
    private static partial void LogCaptureFailed(ILogger logger, Exception exception, string part);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível gravar o estado de monitores e áudio em {Path}; a restauração só vale nesta abertura")]
    private static partial void LogPersistFailed(ILogger logger, string path);

    [LoggerMessage(Level = LogLevel.Information, Message = "Restauração após {Context}: monitores e áudio já estavam como antes")]
    private static partial void LogNothingToRestore(ILogger logger, string context);

    [LoggerMessage(Level = LogLevel.Information, Message = "Restauração após {Context}: {Step}")]
    private static partial void LogStepApplied(ILogger logger, string context, string step);

    [LoggerMessage(Level = LogLevel.Information, Message = "Restauração após {Context}: {What} mantido ({Reason})")]
    private static partial void LogStepSkipped(ILogger logger, string context, string what, string reason);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Restauração após {Context}: falha em {Step}")]
    private static partial void LogStepFailed(ILogger logger, Exception exception, string context, string step);

    [LoggerMessage(Level = LogLevel.Information, Message = "Restauração após {Context} verificada: {Applied} alteração(ões)")]
    private static partial void LogRestoreVerified(ILogger logger, string context, int applied);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Restauração após {Context} incompleta: {Applied} de {Planned} passo(s); ainda diferente: {Remaining}")]
    private static partial void LogRestoreIncomplete(ILogger logger, string context, int applied, int planned, string remaining);

    [LoggerMessage(Level = LogLevel.Information, Message = "Estado de antes de {Context} ({CapturedAt}) não foi restaurado na sessão anterior: {Steps} passo(s) pendente(s)")]
    private static partial void LogPendingFound(ILogger logger, string context, DateTimeOffset capturedAt, int steps);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Estado pendente de monitores e áudio ilegível; descartado: {Path}")]
    private static partial void LogPendingUnreadable(ILogger logger, string path);
}
