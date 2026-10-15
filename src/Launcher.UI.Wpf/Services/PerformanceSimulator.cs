using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Números de desempenho do jogo em execução. Marco 0: valores simulados para validar o layout do HUD e do
/// cartão "Desempenho ao vivo"; a leitura real (PresentMon/ETW) entra depois.
/// </summary>
public sealed partial class PerformanceSimulator : ObservableObject
{
    private const int HistoryLength = 40;
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromMilliseconds(500) };
    private readonly Queue<int> _history = new();

    [ObservableProperty]
    private int _fps;

    [ObservableProperty]
    private double _frameTimeMs;

    [ObservableProperty]
    private int _cpu;

    [ObservableProperty]
    private int _gpu;

    [ObservableProperty]
    private int _gpuTemperature;

    public PerformanceSimulator()
    {
        _timer.Tick += (_, _) => Sample();
    }

    public event EventHandler? Sampled;

    public IReadOnlyCollection<int> History => _history;

    public void Start()
    {
        _history.Clear();
        for (int i = 0; i < HistoryLength; i++)
        {
            Sample();
        }

        _timer.Start();
    }

    public void Stop() => _timer.Stop();

    private void Sample()
    {
#pragma warning disable CA5394 // Valores simulados; não é uso criptográfico.
        Fps = 56 + Random.Shared.Next(0, 7);
        Cpu = Random.Shared.Next(34, 49);
        Gpu = Random.Shared.Next(88, 98);
        GpuTemperature = Random.Shared.Next(64, 72);
#pragma warning restore CA5394
        FrameTimeMs = Math.Round(1000d / Fps, 1);
        _history.Enqueue(Fps);
        while (_history.Count > HistoryLength)
        {
            _history.Dequeue();
        }

        Sampled?.Invoke(this, EventArgs.Empty);
    }
}
