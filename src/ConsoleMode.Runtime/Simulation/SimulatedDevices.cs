using ConsoleMode.Runtime.Steam;
using ConsoleMode.Runtime.Tv;

namespace ConsoleMode.Runtime.Simulation;

/// <summary>Cliente da Steam em memória: testes e teste de fumaça (nunca lê o registro nem fecha janelas).</summary>
public sealed class SimulatedSteamClient : ISteamClient
{
    public bool Running { get; set; }

    /// <summary>Valor de <c>RunningAppID</c>; nulo simula leitura impossível.</summary>
    public int? AppId { get; set; }

    public int TrayRequests { get; private set; }

    public bool IsRunning() => Running;

    public int? RunningAppId() => AppId;

    public bool CloseWindowToTray()
    {
        TrayRequests++;
        return Running;
    }
}

/// <summary>TV em memória: registra os pedidos e pode simular uma TV que não responde.</summary>
public sealed class SimulatedTvControl : ITvControl, ITvDetector
{
    public IList<string> Calls { get; } = [];

    /// <summary>O que "Detectar" encontra (nulo: nenhuma TV respondeu).</summary>
    public TvKind? DetectedKind { get; set; }

    /// <summary>Próximas chamadas falham com esta mensagem.</summary>
    public string? FailWith { get; set; }

    public Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken) => Record($"ligar:{config?.Host}");

    public Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken) => Record($"espera:{config?.Host}");

    public Task<TvKind?> DetectAsync(string host, CancellationToken cancellationToken)
    {
        Calls.Add($"detectar:{host}");
        return Task.FromResult(DetectedKind);
    }

    private Task Record(string call)
    {
        Calls.Add(call);
        return FailWith is { } message ? Task.FromException(new TvControlException(message)) : Task.CompletedTask;
    }
}
