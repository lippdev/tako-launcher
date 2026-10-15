namespace ConsoleMode.Runtime.Tv;

/// <summary>Descobre o tipo da TV num IP (Configurações › TV › Detectar).</summary>
public interface ITvDetector
{
    /// <summary>O tipo reconhecido, ou nulo se nenhuma porta conhecida respondeu (TV desligada, IP errado, recurso desligado).</summary>
    Task<TvKind?> DetectAsync(string host, CancellationToken cancellationToken);
}

/// <summary>O que respondeu no IP: portas abertas e, se houver, a informação da Samsung.</summary>
public sealed record TvProbe(IReadOnlySet<int> OpenPorts, SamsungInfo? Samsung);

/// <summary>Decisão pura a partir do que respondeu (testada com exemplos).</summary>
public static class TvDetection
{
    /// <summary>ADB (Android), API do webOS (LG) e API da Samsung.</summary>
    public static IReadOnlyList<int> Ports { get; } = [AdbProtocol.DefaultPort, WebOsProtocol.PlainPort, WebOsProtocol.SecurePort, SamsungTvProtocol.PlainPort, SamsungTvProtocol.SecurePort];

    /// <summary>
    /// A informação da Samsung é a prova mais forte; depois as portas exclusivas do webOS; depois as da Samsung sem a
    /// informação; por último o ADB (algumas TVs de outras marcas também deixam a 5555 aberta).
    /// </summary>
    public static TvKind? Decide(TvProbe probe)
    {
        ArgumentNullException.ThrowIfNull(probe);
        var open = probe.OpenPorts;
        if (probe.Samsung is { IsSamsung: true })
        {
            return TvKind.Samsung;
        }

        if (open.Contains(WebOsProtocol.SecurePort) || open.Contains(WebOsProtocol.PlainPort))
        {
            return TvKind.LgWebOs;
        }

        if (open.Contains(SamsungTvProtocol.SecurePort) || open.Contains(SamsungTvProtocol.PlainPort))
        {
            return TvKind.Samsung;
        }

        return open.Contains(AdbProtocol.DefaultPort) ? TvKind.Android : null;
    }
}

/// <summary>Testa as portas conhecidas em paralelo e, se a 8001 responde, lê a informação da Samsung.</summary>
public sealed class TvDetector(ITvTransport transport) : ITvDetector
{
    public static readonly TimeSpan PortTimeout = TimeSpan.FromSeconds(2);

    public async Task<TvKind?> DetectAsync(string host, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(host);
        string name = AdbProtocol.ParseEndpoint(host).Host;
        if (name.Length == 0)
        {
            return null;
        }

        var checks = TvDetection.Ports.Select(async port => (port, open: await transport.IsPortOpenAsync(name, port, PortTimeout, cancellationToken).ConfigureAwait(false)));
        var results = await Task.WhenAll(checks).ConfigureAwait(false);
        var open = results.Where(r => r.open).Select(r => r.port).ToHashSet();
        SamsungInfo? samsung = null;
        if (open.Contains(SamsungTvProtocol.PlainPort))
        {
            try
            {
                samsung = SamsungTvProtocol.ParseInfo(await transport.GetStringAsync(SamsungTvProtocol.InfoUri(name), cancellationToken).ConfigureAwait(false));
            }
            catch (TvConnectionException)
            {
                // Porta aberta sem a API: decide só pelas portas.
            }
        }

        return TvDetection.Decide(new TvProbe(open, samsung));
    }
}

/// <summary>
/// O controle de TV da sessão: escolhe Android, LG ou Samsung pelo <see cref="TvControlConfig.Kind"/> e detecta o tipo
/// pelas portas.
/// </summary>
public sealed class TvControlRouter(ITvControl android, ITvControl lg, ITvControl samsung, ITvDetector detector) : ITvControl, ITvDetector
{
    public Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken) =>
        For(config).TurnOnAsync(config, approvalTimeout, cancellationToken);

    public Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken) =>
        For(config).StandbyAsync(config, cancellationToken);

    public Task<TvKind?> DetectAsync(string host, CancellationToken cancellationToken) => detector.DetectAsync(host, cancellationToken);

    private ITvControl For(TvControlConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        return config.Kind switch
        {
            TvKind.LgWebOs => lg,
            TvKind.Samsung => samsung,
            _ => android,
        };
    }
}
