using Launcher.Core.Session;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Acompanha um jogo real (MVP-026): lê a lista de processos a cada <see cref="PollInterval"/> fora da thread da UI,
/// aplica no <see cref="GameRunTracker"/> e avisa na thread de quem chamou <see cref="Start"/> quando o status muda.
/// </summary>
public sealed class GameProcessMonitor(TimeProvider time) : IDisposable
{
    public static readonly TimeSpan PollInterval = TimeSpan.FromSeconds(2);

    private readonly ProcessSnapshot _snapshot = new();
    private CancellationTokenSource? _cancel;

    /// <summary>Jogo acompanhado agora (nulo quando parado).</summary>
    public GameRunTracker? Tracker { get; private set; }

    public bool IsTracking => Tracker is { IsFinished: false };

    /// <summary>Começa a acompanhar; <paramref name="changed"/> roda a cada mudança de status, até o jogo terminar.</summary>
    public void Start(GameRunTracker tracker, Action<GameRunTracker> changed)
    {
        ArgumentNullException.ThrowIfNull(tracker);
        Stop();
        Tracker = tracker;
        _cancel = new CancellationTokenSource();
        _ = PollAsync(tracker, changed, _cancel.Token);
    }

    public void Stop()
    {
        _cancel?.Cancel();
        _cancel?.Dispose();
        _cancel = null;
        Tracker = null;
    }

    /// <summary>"Voltar ao jogo": traz a janela do jogo para frente, se houver uma.</summary>
    public bool TryBringGameToFront() => Tracker is { } tracker && ProcessSnapshot.TryBringToFront(tracker.GameProcessIds);

    public void Dispose() => Stop();

    private async Task PollAsync(GameRunTracker tracker, Action<GameRunTracker> changed, CancellationToken cancel)
    {
        using var timer = new PeriodicTimer(PollInterval, time);
        try
        {
            // ConfigureAwait(true): o tracker e o aviso ficam na thread da UI; só a leitura dos processos vai para o pool.
            while (await timer.WaitForNextTickAsync(cancel).ConfigureAwait(true))
            {
                var processes = await Task.Run(_snapshot.Take, cancel).ConfigureAwait(true);
                if (cancel.IsCancellationRequested)
                {
                    return;
                }

                var before = tracker.Status;
                if (tracker.Update(processes, time.GetUtcNow()) != before)
                {
                    changed(tracker);
                }

                if (tracker.IsFinished)
                {
                    return;
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Parado (outro jogo, app fechando).
        }
    }
}
