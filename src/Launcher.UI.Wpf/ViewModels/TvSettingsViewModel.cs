// Adaptado de lippdev/consolemode: src/ConsoleMode/ViewModels/MainViewModel.Tv.cs
using System.IO;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Tv;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › TV (MVP-002): Google TV / Android TV pela rede (ADB). Campos de texto pelo teclado; o controle
/// navega entre campos e botões. "Testar" liga a TV e troca a entrada agora e, na primeira vez, pareia (a TV pede
/// "Permitir depuração").
/// </summary>
public sealed partial class TvSettingsViewModel : ObservableObject, IOverlayViewModel
{
    private readonly Func<TvControlConfig, Task> _test;
    private readonly Action<TvControlConfig> _save;
    private readonly Action _cancel;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(EnabledLabel))]
    private bool _enabled;

    [ObservableProperty]
    private string _host;

    [ObservableProperty]
    private string _macAddress;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HdmiLabel))]
    private int _hdmiInput;

    [ObservableProperty]
    private string _inputCommand;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(StandbyLabel))]
    private bool _standbyAfterGame;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(TestLabel))]
    private bool _isTesting;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    private bool _statusIsError;

    public TvSettingsViewModel(TvControlConfig config, Func<TvControlConfig, Task> test, Action<TvControlConfig> save, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(config);
        _test = test;
        _save = save;
        _cancel = cancel;
        _enabled = config.Enabled;
        _host = config.Host;
        _macAddress = config.MacAddress;
        _hdmiInput = Math.Clamp(config.HdmiInput, 1, 4);
        _inputCommand = config.InputCommand;
        _standbyAfterGame = config.StandbyAfterGame;
    }

    public string EnabledLabel => Enabled ? "Ligar a TV ao abrir um jogo: sim" : "Ligar a TV ao abrir um jogo: não";

    public string HdmiLabel => $"Entrada do PC: HDMI {HdmiInput}";

    public string StandbyLabel => StandbyAfterGame ? "Colocar a TV em espera ao sair do jogo: sim" : "Colocar a TV em espera ao sair do jogo: não";

    public string TestLabel => IsTesting ? "Falando com a TV…" : "Testar";

    public bool HasStatus => Status is not null;

    [RelayCommand]
    private void ToggleEnabled() => Enabled = !Enabled;

    [RelayCommand]
    private void CycleHdmi() => HdmiInput = (HdmiInput % 4) + 1;

    [RelayCommand]
    private void ToggleStandby() => StandbyAfterGame = !StandbyAfterGame;

    [RelayCommand]
    private async Task TestAsync()
    {
        if (IsTesting)
        {
            return;
        }

        if (Validate(requireHost: true) is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        IsTesting = true;
        ShowStatus("Falando com a TV… Se ela pedir, escolha \"Sempre permitir\" e \"Permitir\" na tela da TV.", error: false);
        try
        {
            await _test(Build()).ConfigureAwait(true);
            ShowStatus("A TV respondeu: ela deve estar ligada na entrada do PC.", error: false);
        }
        catch (TvControlException ex)
        {
            ShowStatus(ex.Message, error: true);
        }
        catch (Exception ex) when (ex is IOException or System.Net.Sockets.SocketException or InvalidDataException or System.Security.Cryptography.CryptographicException or UnauthorizedAccessException or ConsoleMode.Runtime.RuntimeEnvironmentException)
        {
            ShowStatus($"Não foi possível falar com a TV: {ex.Message}", error: true);
        }
        finally
        {
            IsTesting = false;
        }
    }

    [RelayCommand]
    private void Save()
    {
        if (Validate(requireHost: Enabled) is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        _save(Build());
    }

    [RelayCommand]
    private void Cancel() => _cancel();

    /// <summary>Configuração montada a partir dos campos (sem espaços nas pontas).</summary>
    public TvControlConfig Build() => new()
    {
        Enabled = Enabled,
        Host = Host.Trim(),
        MacAddress = MacAddress.Trim(),
        HdmiInput = HdmiInput,
        InputCommand = InputCommand.Trim(),
        StandbyAfterGame = StandbyAfterGame,
    };

    private string? Validate(bool requireHost)
    {
        if (requireHost && string.IsNullOrWhiteSpace(Host))
        {
            return "Informe o endereço IP da TV (Configurações › Rede, na TV).";
        }

        if (!string.IsNullOrWhiteSpace(MacAddress) && !WakeOnLan.TryParseMac(MacAddress, out _))
        {
            return "Endereço MAC inválido. Use o formato AA:BB:CC:DD:EE:FF ou deixe vazio.";
        }

        return null;
    }

    private void ShowStatus(string text, bool error)
    {
        Status = text;
        StatusIsError = error;
    }
}
