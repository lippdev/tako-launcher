using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Steam;
using ConsoleMode.Runtime.Tv;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Session.Devices;

/// <summary>O que fazer com áudio, Steam e TV numa sessão de jogo (lido das Configurações na hora de abrir o jogo).</summary>
public sealed record SessionDeviceOptions(AudioOutputChoice? GameAudio, bool SteamToTray, TvControlConfig Tv)
{
    public static SessionDeviceOptions Off { get; } = new(null, false, TvControlConfig.Default);

    /// <summary>Nada a fazer no início da sessão.</summary>
    public bool IsIdle => GameAudio is null && !Tv.IsActive;
}

/// <summary>Resumo do fim da sessão, para o aviso de "bem-vindo".</summary>
public sealed record SessionDevicesReport(bool AudioRestored, SteamTrayDecision Steam, bool SteamSentToTray);

/// <summary>
/// MVP-002 (parte 2): o que acompanha uma sessão de jogo além de monitores e áudio restaurados (MVP-012).
/// <list type="bullet">
/// <item>Início: liga a TV e troca para a entrada do PC (se configurada), depois troca a saída de áudio para a escolhida
/// (esperando ela aparecer, já que o áudio HDMI da TV só surge com a TV ligada).</item>
/// <item>Fim: devolve a saída de áudio, manda a Steam para a bandeja (se pedido e sem outro jogo rodando) e coloca a TV
/// em espera (se pedido).</item>
/// </list>
/// Nada aqui derruba ou atrasa a sessão: falhas vão para o log. O início roda fora da thread da UI.
/// </summary>
public sealed partial class GameSessionDevices(
    IAudioEndpointService audio,
    ISteamClient steam,
    ITvControl tv,
    TimeProvider time,
    ILogger<GameSessionDevices> logger)
{
    /// <summary>Uma TV acordando de espera profunda (Wake-on-LAN) precisa de tempo para responder.</summary>
    public static readonly TimeSpan TvStartTimeout = TimeSpan.FromSeconds(45);

    public static readonly TimeSpan TvStandbyTimeout = TimeSpan.FromSeconds(15);

    /// <summary>Configurações › Testar: tempo para tocar em "Permitir" na TV na primeira vez.</summary>
    public static readonly TimeSpan PairingTimeout = TimeSpan.FromSeconds(60);

    public static readonly TimeSpan TestTimeout = TimeSpan.FromSeconds(90);

    /// <summary>Quanto esperar a saída de áudio escolhida aparecer depois de ligar a TV.</summary>
    public static readonly TimeSpan AudioDeviceWait = TimeSpan.FromSeconds(15);

    private static readonly TimeSpan AudioPoll = TimeSpan.FromSeconds(1);

    private readonly AudioOutputSwitch _switch = new(audio);
    private readonly Lock _gate = new();
    private CancellationTokenSource? _begin;
    private Task _beginTask = Task.CompletedTask;
    private AudioOutputSwitchRecord? _audioRecord;
    private SessionDeviceOptions _options = SessionDeviceOptions.Off;

    /// <summary>Volume e saída de áudio agora, para a Central de Controle e as Configurações.</summary>
    public AudioVolumeControl Volume { get; } = new(audio);

    /// <summary><c>RunningAppID</c> da Steam agora (para o acompanhamento do jogo); nulo se não der para ler.</summary>
    public int? SteamRunningAppId() => steam.RunningAppId();

    /// <summary>
    /// Liga a TV e troca a entrada agora, esperando o fim (com prazo). Usado antes de focar a "Tela de jogo", que
    /// precisa da TV ligada; nunca lança.
    /// </summary>
    public Task WakeTvAsync(string gameId, TvControlConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        return config.IsActive
            ? RunTvAsync(gameId, "ligar", TvStartTimeout, token => tv.TurnOnAsync(config, TimeSpan.FromSeconds(10), token))
            : Task.CompletedTask;
    }

    /// <summary>
    /// Início da sessão: dispara TV e áudio em segundo plano e volta na hora. <paramref name="tvReady"/>: a TV já foi
    /// ligada por <see cref="WakeTvAsync"/> (não liga de novo; a espera ao sair continua valendo).
    /// </summary>
    public Task BeginSession(string gameId, SessionDeviceOptions options, bool tvReady = false)
    {
        ArgumentNullException.ThrowIfNull(options);
        CancelPending();
        lock (_gate)
        {
            _options = options;
            _audioRecord = null;
            if (options.GameAudio is null && (tvReady || !options.Tv.IsActive))
            {
                return Task.CompletedTask;
            }

            var cancel = new CancellationTokenSource();
            _begin = cancel;
            _beginTask = Task.Run(() => RunBeginAsync(gameId, options, tvReady, cancel.Token), CancellationToken.None);
            return _beginTask;
        }
    }

    /// <summary>
    /// Fim da sessão (estado Restoring, antes da restauração de monitores e áudio): devolve a saída de áudio, decide a
    /// bandeja da Steam e, com <paramref name="gameEnded"/>, coloca a TV em espera. Sem <paramref name="gameEnded"/> (o
    /// jogo não abriu), só desfaz o áudio.
    /// </summary>
    /// <param name="steamAppId">Id do jogo na Steam, quando há.</param>
    public SessionDevicesReport EndSession(string gameId, bool steamGame, int? steamAppId, bool gameEnded)
    {
        CancelPending();
        AudioOutputSwitchRecord? record;
        SessionDeviceOptions options;
        lock (_gate)
        {
            record = _audioRecord;
            options = _options;
            _audioRecord = null;
            _options = SessionDeviceOptions.Off;
        }

        bool audioRestored = record is not null && RestoreAudio(gameId, record);
        var decision = SteamTrayDecision.NotSteamGame;
        bool sentToTray = false;
        if (gameEnded)
        {
            (decision, sentToTray) = SendSteamToTray(gameId, steamGame, steamAppId, options.SteamToTray);
            if (options.Tv is { IsActive: true, StandbyAfterGame: true } config)
            {
                _ = RunTvAsync(gameId, "colocar em espera", TvStandbyTimeout, token => tv.StandbyAsync(config, token));
            }
        }

        return new SessionDevicesReport(audioRestored, decision, sentToTray);
    }

    /// <summary>Configurações › Testar: liga a TV e troca a entrada agora. Erros para o usuário em <see cref="TvControlException"/>.</summary>
    public async Task TestTvAsync(TvControlConfig config, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        if (string.IsNullOrWhiteSpace(config.Host))
        {
            throw new TvControlException("Informe o endereço IP da TV.");
        }

        LogTvTest(logger);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TestTimeout);
        try
        {
            await tv.TurnOnAsync(config, PairingTimeout, timeout.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new TvControlException($"A TV não terminou de responder em {(int)TestTimeout.TotalSeconds} segundos. Tente de novo.");
        }
    }

    /// <summary>
    /// Configurações › Detectar: o tipo da TV no IP (Android, LG ou Samsung) pelas portas que respondem; nulo se nenhuma
    /// respondeu ou se o controle de TV não sabe detectar.
    /// </summary>
    public async Task<TvKind?> DetectTvAsync(string host, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(host);
        if (tv is not ITvDetector detector || string.IsNullOrWhiteSpace(host))
        {
            return null;
        }

        var kind = await detector.DetectAsync(host.Trim(), cancellationToken).ConfigureAwait(false);
        LogTvDetected(logger, kind);
        return kind;
    }

    /// <summary>Linha de log para um evento da TV (usado pela composição ao criar o controle da TV).</summary>
    public static void LogTvEvent(ILogger logger, TvEvent tvEvent)
    {
        ArgumentNullException.ThrowIfNull(logger);
        switch (tvEvent)
        {
            case TvEvent.WakeOnLanSent:
                LogTvWakeOnLan(logger);
                break;
            case TvEvent.WaitingForApproval:
                LogTvWaitingApproval(logger);
                break;
            case TvEvent.KeyCreated:
                LogTvKeyCreated(logger);
                break;
            case TvEvent.PairingSaved:
                LogTvPairingSaved(logger);
                break;
        }
    }

    private async Task RunBeginAsync(string gameId, SessionDeviceOptions options, bool tvReady, CancellationToken cancellationToken)
    {
        try
        {
            if (options.Tv.IsActive && !tvReady)
            {
                var config = options.Tv;
                await RunTvAsync(gameId, "ligar", TvStartTimeout, token => tv.TurnOnAsync(config, TimeSpan.FromSeconds(10), token), cancellationToken).ConfigureAwait(false);
            }

            if (options.GameAudio is { } choice)
            {
                await SwitchAudioAsync(gameId, choice, options.Tv.IsActive ? AudioDeviceWait : TimeSpan.Zero, cancellationToken).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            // A sessão terminou antes: o fim desfaz o que já mudou.
        }
    }

    private async Task SwitchAudioAsync(string gameId, AudioOutputChoice choice, TimeSpan wait, CancellationToken cancellationToken)
    {
        var deadline = time.GetUtcNow() + wait;
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                var (outcome, record) = _switch.Apply(choice);
                if (outcome == AudioSwitchOutcome.DeviceMissing && time.GetUtcNow() < deadline)
                {
                    await Task.Delay(AudioPoll, time, cancellationToken).ConfigureAwait(false);
                    continue;
                }

                lock (_gate)
                {
                    _audioRecord = record;
                }

                if (outcome == AudioSwitchOutcome.Switched)
                {
                    LogAudioSwitched(logger, gameId, choice.Name);
                }
                else
                {
                    LogAudioNotSwitched(logger, gameId, choice.Name, outcome);
                }

                return;
            }
            catch (RuntimeEnvironmentException ex)
            {
                LogAudioFailed(logger, ex, gameId, choice.Name);
                return;
            }
        }
    }

    private bool RestoreAudio(string gameId, AudioOutputSwitchRecord record)
    {
        try
        {
            int restored = _switch.Restore(record);
            LogAudioRestored(logger, gameId, restored);
            return restored > 0;
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogAudioRestoreFailed(logger, ex, gameId);
            return false;
        }
    }

    private (SteamTrayDecision Decision, bool Sent) SendSteamToTray(string gameId, bool steamGame, int? steamAppId, bool settingOn)
    {
        bool ask = steamGame && settingOn;
        int? running = ask ? steam.RunningAppId() : null;

        // O próprio jogo ainda no registro (a Steam demora alguns segundos para zerar) não conta como outro jogo.
        if (running is { } id && id == steamAppId)
        {
            running = 0;
        }

        var decision = SteamClientRules.DecideTray(steamGame, settingOn, ask && steam.IsRunning(), running);
        bool sent = decision == SteamTrayDecision.CloseToTray && steam.CloseWindowToTray();
        if (steamGame)
        {
            LogSteamTray(logger, gameId, decision, sent);
        }

        return (decision, sent);
    }

    private async Task RunTvAsync(string gameId, string action, TimeSpan timeout, Func<CancellationToken, Task> work, CancellationToken cancellationToken = default)
    {
        var started = time.GetUtcNow();
        using var limit = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        limit.CancelAfter(timeout);
        try
        {
            await work(limit.Token).ConfigureAwait(false);
            double seconds = (time.GetUtcNow() - started).TotalSeconds;
            LogTvDone(logger, gameId, action, seconds);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            LogTvTimedOut(logger, gameId, action, timeout.TotalSeconds);
        }
        catch (Exception ex) when (ex is TvControlException or IOException or System.Net.Sockets.SocketException or InvalidDataException or System.Security.Cryptography.CryptographicException or UnauthorizedAccessException or RuntimeEnvironmentException)
        {
            LogTvFailed(logger, ex, gameId, action);
        }
    }

    private void CancelPending()
    {
        CancellationTokenSource? begin;
        lock (_gate)
        {
            begin = _begin;
            _begin = null;
        }

        if (begin is null)
        {
            return;
        }

        begin.Cancel();
        try
        {
            // A troca de áudio em andamento termina antes de o fim ler o registro (alguns ms; a TV respeita o cancelamento).
            _ = _beginTask.Wait(TimeSpan.FromSeconds(2));
        }
        catch (AggregateException)
        {
            // Já registrado no log pela própria tarefa.
        }

        begin.Dispose();
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Áudio de {GameId}: saída padrão trocada para {Device}")]
    private static partial void LogAudioSwitched(ILogger logger, string gameId, string device);

    [LoggerMessage(Level = LogLevel.Information, Message = "Áudio de {GameId}: saída {Device} não aplicada ({Outcome})")]
    private static partial void LogAudioNotSwitched(ILogger logger, string gameId, string device, AudioSwitchOutcome outcome);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Áudio de {GameId}: falha ao trocar a saída para {Device}")]
    private static partial void LogAudioFailed(ILogger logger, Exception exception, string gameId, string device);

    [LoggerMessage(Level = LogLevel.Information, Message = "Áudio de {GameId}: saída de antes devolvida em {Roles} papel(éis)")]
    private static partial void LogAudioRestored(ILogger logger, string gameId, int roles);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Áudio de {GameId}: falha ao devolver a saída de antes")]
    private static partial void LogAudioRestoreFailed(ILogger logger, Exception exception, string gameId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Steam após {GameId}: {Decision} (bandeja pedida: {Sent})")]
    private static partial void LogSteamTray(ILogger logger, string gameId, SteamTrayDecision decision, bool sent);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV ({GameId}): {Action} ok em {Seconds:0.0}s")]
    private static partial void LogTvDone(ILogger logger, string gameId, string action, double seconds);

    [LoggerMessage(Level = LogLevel.Warning, Message = "TV ({GameId}): {Action} sem resposta em {Seconds:0}s")]
    private static partial void LogTvTimedOut(ILogger logger, string gameId, string action, double seconds);

    [LoggerMessage(Level = LogLevel.Warning, Message = "TV ({GameId}): falha ao {Action}")]
    private static partial void LogTvFailed(ILogger logger, Exception exception, string gameId, string action);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: teste pelas Configurações")]
    private static partial void LogTvTest(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: sem resposta; Wake-on-LAN enviado")]
    private static partial void LogTvWakeOnLan(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: este PC ainda não está autorizado; aguardando \"Permitir\" na TV")]
    private static partial void LogTvWaitingApproval(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: chave ADB nova criada (a TV vai pedir permissão)")]
    private static partial void LogTvKeyCreated(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: a TV aceitou este PC; chave de pareamento guardada (cifrada)")]
    private static partial void LogTvPairingSaved(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "TV: detecção pelas portas encontrou {Kind} (vazio = nenhuma)")]
    private static partial void LogTvDetected(ILogger logger, TvKind? kind);
}
