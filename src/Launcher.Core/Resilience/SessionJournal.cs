using System.Text.Json;

namespace Launcher.Core.Resilience;

/// <summary>
/// Marcador de "sessão em andamento" gravado na abertura. <see cref="Running"/> continua verdadeiro se o Tako não
/// fechou limpo (travamento, processo morto, queda de energia); <see cref="UncleanExits"/> conta as saídas sujas
/// seguidas até a abertura que gravou o marcador.
/// </summary>
public sealed record SessionMarker(bool Running, int UncleanExits, DateTimeOffset? StartedAt);

/// <summary>Por que a abertura entrou (ou não) no modo seguro.</summary>
public enum StartupReason
{
    Normal,

    /// <summary>As últimas aberturas não fecharam limpo (proteção contra loop de travamento).</summary>
    CrashLoop,

    /// <summary>Pedido explícito (<c>--safe-mode</c>).</summary>
    Requested,
}

/// <summary>Decisão da abertura: modo seguro ou normal, e o marcador a gravar.</summary>
public sealed record StartupDecision(bool SafeMode, StartupReason Reason, int UncleanExits, SessionMarker Marker);

/// <summary>
/// Proteção contra loop de travamento (MVP-021/036), sem E/S: se as <see cref="UncleanExitsForSafeMode"/> últimas
/// aberturas não fecharam limpo, a próxima abre em modo seguro (tema embutido, sem tema do disco nem atualizações
/// automáticas).
/// </summary>
public static class CrashLoopPolicy
{
    public const int UncleanExitsForSafeMode = 2;

    public static StartupDecision Decide(SessionMarker? previous, bool safeModeRequested, DateTimeOffset now)
    {
        int unclean = previous is { Running: true } ? Math.Max(0, previous.UncleanExits) + 1 : 0;
        var reason = (safeModeRequested, unclean >= UncleanExitsForSafeMode) switch
        {
            (true, _) => StartupReason.Requested,
            (_, true) => StartupReason.CrashLoop,
            _ => StartupReason.Normal,
        };
        return new StartupDecision(reason != StartupReason.Normal, reason, unclean, new SessionMarker(true, unclean, now));
    }

    /// <summary>Marcador gravado no fechamento limpo: zera a contagem.</summary>
    public static SessionMarker CleanExit() => new(false, 0, null);

    /// <summary>"Voltar ao normal": a sessão continua em andamento, mas sem herdar as falhas anteriores.</summary>
    public static SessionMarker Forgive(SessionMarker current) => current with { UncleanExits = 0 };
}

/// <summary>
/// Arquivo do marcador (<c>session.json</c> na pasta de dados). Falhas de leitura e gravação nunca impedem a abertura:
/// sem conseguir gravar, o Tako abre normalmente (só perde a proteção nesta sessão).
/// </summary>
public sealed class SessionJournal(string path, TimeProvider time)
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    private SessionMarker? _current;

    public string FilePath { get; } = path;

    /// <summary>Lê o marcador anterior, decide o modo da abertura e grava "sessão em andamento".</summary>
    public StartupDecision Begin(bool safeModeRequested)
    {
        var decision = CrashLoopPolicy.Decide(Read(), safeModeRequested, time.GetUtcNow());
        _current = decision.Marker;
        TryWrite(decision.Marker);
        return decision;
    }

    /// <summary>Fechamento limpo: apaga a sessão em andamento.</summary>
    public void EndClean()
    {
        _current = CrashLoopPolicy.CleanExit();
        TryWrite(_current);
    }

    /// <summary>Sair do modo seguro nesta sessão: as falhas anteriores deixam de contar.</summary>
    public void Forgive()
    {
        _current = CrashLoopPolicy.Forgive(_current ?? new SessionMarker(true, 0, time.GetUtcNow()));
        TryWrite(_current);
    }

    /// <summary>Marcador gravado; arquivo ilegível conta como uma sessão que não fechou limpo.</summary>
    public SessionMarker? Read()
    {
        try
        {
            return File.Exists(FilePath) ? JsonSerializer.Deserialize<SessionMarker>(File.ReadAllText(FilePath), Options) ?? Unreadable : null;
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Arquivo truncado é sinal de que o processo morreu no meio da gravação.
            return Unreadable;
        }
    }

    private static SessionMarker Unreadable => new(true, 0, null);

    private bool TryWrite(SessionMarker marker)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
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
}
