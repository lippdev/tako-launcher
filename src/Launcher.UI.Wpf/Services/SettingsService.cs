using CommunityToolkit.Mvvm.ComponentModel;

namespace Launcher.UI.Wpf.Services;

/// <summary>Preferências do usuário. No Marco 0 ficam em memória; persistência em SQLite é o MVP-005.</summary>
public sealed partial class SettingsService : ObservableObject
{
    [ObservableProperty]
    private bool _dynamicColor = true;

    [ObservableProperty]
    private bool _reducedMotion = SystemParametersReducedMotion();

    [ObservableProperty]
    private bool _startFullscreenOnLogin = true;

    public TimeSpan FocusDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(160);

    public TimeSpan PageDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(280);

    public TimeSpan ColorDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(600);

    // O Windows expõe "mostrar animações" em SystemParameters.ClientAreaAnimation.
    private static bool SystemParametersReducedMotion() => !System.Windows.SystemParameters.ClientAreaAnimation;
}

/// <summary>Opções da sessão de jogo ajustadas pela Central de Controle.</summary>
public sealed partial class SessionOptions : ObservableObject
{
    public static readonly string[] PerfModes = ["Desligado", "Só FPS", "Completo"];
    public static readonly string[] Outputs = ["TV (HDMI)", "Fone Bluetooth", "Alto-falantes"];
    public static readonly string[] Resolutions = ["3840 × 2160", "2560 × 1440", "1920 × 1080"];

    [ObservableProperty]
    private int _perfMode;

    [ObservableProperty]
    private int _volume = 70;

    [ObservableProperty]
    private int _output;

    [ObservableProperty]
    private int _resolution;

    [ObservableProperty]
    private bool _hdr = true;

    [ObservableProperty]
    private int _captures;
}
