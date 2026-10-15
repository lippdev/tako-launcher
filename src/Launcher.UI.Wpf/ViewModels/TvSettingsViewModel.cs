// Adaptado de lippdev/consolemode: src/ConsoleMode/ViewModels/MainViewModel.Tv.cs
using System.IO;
using System.Net;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Tv;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › TV (MVP-002): Google TV / Android TV (ADB), LG (webOS) ou Samsung (Tizen) pela rede. Campos de
/// texto pelo teclado; o controle navega entre campos e botões. "Detectar" descobre o tipo pelo IP; "Testar" liga a TV e
/// troca a entrada agora e, na primeira vez, pareia (a TV pede para aceitar este PC).
/// </summary>
public sealed partial class TvSettingsViewModel : ObservableObject, IOverlayViewModel
{
    private readonly Func<TvControlConfig, Task> _test;
    private readonly Func<string, Task<TvKind?>> _detect;
    private readonly Action<TvControlConfig> _save;
    private readonly Action _cancel;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(EnabledLabel))]
    private bool _enabled;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(KindLabel), nameof(Help), nameof(IsAndroid), nameof(HostLabel))]
    private TvKind _kind;

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
    [NotifyPropertyChangedFor(nameof(DetectLabel))]
    private bool _isDetecting;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    private bool _statusIsError;

    public TvSettingsViewModel(TvControlConfig config, Func<TvControlConfig, Task> test, Func<string, Task<TvKind?>> detect, Action<TvControlConfig> save, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(config);
        _test = test;
        _detect = detect;
        _kind = config.Kind;
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

    public string DetectLabel => IsDetecting ? "Procurando…" : "Detectar pelo IP";

    public string KindLabel => $"Tipo: {TvKinds.Label(Kind)}";

    /// <summary>Só o Android tem o comando de entrada avançado (shell).</summary>
    public bool IsAndroid => Kind == TvKind.Android;

    public string HostLabel => IsAndroid ? "IP da TV (Configurações › Rede, na TV)" : "IP da TV (ex.: 192.168.0.50)";

    /// <summary>O que ligar na TV para o Tako conseguir falar com ela.</summary>
    public string Help => Kind switch
    {
        TvKind.LgWebOs => "Pela rede, sem instalar nada. Na TV LG: ligue \"LG Connect Apps\" e, para ligar a TV desligada, \"Ligar via Wi-Fi\" (preencha o MAC). Na primeira vez, aceite o aviso do Tako na TV.",
        TvKind.Samsung => "Pela rede, sem instalar nada. Na TV Samsung: Gerenciador de dispositivos externos › Gerenciador de conexão de dispositivo › \"Acesso de dispositivos IP remotos\" ligado. Na primeira vez, escolha \"Permitir\" para o Tako na TV.",
        _ => "Pela rede, sem instalar nada. Na TV: Configurações › Sistema › Opções do desenvolvedor › ative \"Depuração USB\" (ou \"Depuração pela rede\"). Na primeira vez, aceite \"Permitir depuração\" na TV.",
    };

    public bool HasStatus => Status is not null;

    [RelayCommand]
    private void ToggleEnabled() => Enabled = !Enabled;

    [RelayCommand]
    private void CycleKind() => Kind = TvKinds.Next(Kind);

    [RelayCommand]
    private async Task DetectAsync()
    {
        if (IsDetecting || IsTesting)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(Host))
        {
            ShowStatus("Informe o endereço IP da TV para detectar o tipo.", error: true);
            return;
        }

        IsDetecting = true;
        ShowStatus("Procurando a TV nesse IP…", error: false);
        try
        {
            if (await _detect(Host.Trim()).ConfigureAwait(true) is { } found)
            {
                Kind = found;
                ShowStatus($"Encontrada: {TvKinds.Label(found)}. Toque em Testar para parear.", error: false);
            }
            else
            {
                ShowStatus("Nenhuma TV conhecida respondeu nesse IP. Confira se a TV está ligada e na mesma rede, e se o controle pela rede está ativado nela (LG: \"LG Connect Apps\"; Samsung: \"Acesso de dispositivos IP remotos\"; Android: \"Depuração USB\").", error: true);
            }
        }
        catch (Exception ex) when (ex is IOException or System.Net.Sockets.SocketException)
        {
            ShowStatus($"Não foi possível procurar a TV: {ex.Message}", error: true);
        }
        finally
        {
            IsDetecting = false;
        }
    }

    [RelayCommand]
    private void CycleHdmi() => HdmiInput = (HdmiInput % 4) + 1;

    [RelayCommand]
    private void ToggleStandby() => StandbyAfterGame = !StandbyAfterGame;

    [RelayCommand]
    private async Task TestAsync()
    {
        if (IsTesting || IsDetecting)
        {
            return;
        }

        if (Validate(requireHost: true) is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        IsTesting = true;
        ShowStatus(Kind switch
        {
            TvKind.LgWebOs => "Falando com a TV… Se ela mostrar um aviso, escolha \"Aceitar\" na tela da TV.",
            TvKind.Samsung => "Falando com a TV… Se ela perguntar, escolha \"Permitir\" na tela da TV.",
            _ => "Falando com a TV… Se ela pedir, escolha \"Sempre permitir\" e \"Permitir\" na tela da TV.",
        }, error: false);
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
        Kind = Kind,
        Host = Host.Trim(),
        MacAddress = MacAddress.Trim(),
        HdmiInput = HdmiInput,
        InputCommand = IsAndroid ? InputCommand.Trim() : string.Empty,
        StandbyAfterGame = StandbyAfterGame,
    };

    private string? Validate(bool requireHost)
    {
        if (requireHost && string.IsNullOrWhiteSpace(Host))
        {
            return "Informe o endereço IP da TV (Configurações › Rede, na TV).";
        }

        if (!IsAndroid && !string.IsNullOrWhiteSpace(Host) && !IPAddress.TryParse(Host.Trim(), out _))
        {
            return "Para TVs LG e Samsung, use o IP da TV (ex.: 192.168.0.50), não um nome.";
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
