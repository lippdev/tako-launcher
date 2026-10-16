using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library.Metadata.SteamGridDb;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Capas e metadados › SteamGridDB (ADR-0023): a chave gratuita da API do próprio usuário, como a IGDB e
/// o ScreenScraper. Passo a passo na tela, "Colar" para quem usa só o controle, "Testar" com mensagem clara e "Salvar". A
/// chave salva nunca volta para a tela: o campo vazio mantém a que já está salva.
/// </summary>
public sealed partial class SteamGridDbSettingsViewModel : ObservableObject, IOverlayViewModel
{
    public const string SiteUrl = "https://www.steamgriddb.com/profile/preferences/api";

    private readonly SteamGridDbSettings _saved;
    private readonly Func<SteamGridDbSettings, Task> _test;
    private readonly Action<SteamGridDbSettings> _save;
    private readonly Action _openSite;
    private readonly Func<string?> _readClipboard;
    private readonly Action _cancel;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(SecretToggleLabel))]
    private bool _isSecretVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(EnabledLabel))]
    private bool _enabled;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(AutoFillLabel))]
    private bool _autoFill;

    [ObservableProperty]
    private string _apiKey = string.Empty;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(TestLabel))]
    private bool _isTesting;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    private bool _statusIsError;

    public SteamGridDbSettingsViewModel(SteamGridDbSettings saved, Func<SteamGridDbSettings, Task> test, Action<SteamGridDbSettings> save, Action openSite, Func<string?> readClipboard, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(saved);
        _saved = saved;
        _test = test;
        _save = save;
        _openSite = openSite;
        _readClipboard = readClipboard;
        _cancel = cancel;
        _enabled = saved.Enabled;
        _autoFill = saved.AutoFill;
    }

    public string EnabledLabel => Enabled ? "Usar o SteamGridDB: sim" : "Usar o SteamGridDB: não";

    public string AutoFillLabel => AutoFill ? "Usar quando faltar capa: sim" : "Usar quando faltar capa: não";

    public string TestLabel => IsTesting ? "Testando…" : "Testar";

    public string KeyLabel => _saved.ApiKey.Length > 0 ? "Chave da API (já salva; deixe vazio para manter)" : "Chave da API";

    public IReadOnlyList<string> Steps { get; } =
    [
        "1. Crie uma conta grátis em steamgriddb.com (entrar com a Steam funciona).",
        "2. Abra Preferências › API (o botão abaixo leva direto para lá).",
        "3. Toque em \"Generate API Key\", copie a chave e toque em Colar ao lado do campo. Depois, Testar e Salvar.",
    ];

    public bool HasStatus => Status is not null;

    public string SecretToggleLabel => IsSecretVisible ? "Ocultar" : "Mostrar";

    [RelayCommand]
    private void ToggleSecretVisible() => IsSecretVisible = !IsSecretVisible;

    [RelayCommand]
    private void ToggleEnabled() => Enabled = !Enabled;

    [RelayCommand]
    private void ToggleAutoFill() => AutoFill = !AutoFill;

    [RelayCommand]
    private void OpenSite()
    {
        _openSite();
        ShowStatus("Abrimos o SteamGridDB no navegador. Gere a chave e volte aqui para colar.", error: false);
    }

    [RelayCommand]
    private void PasteKey()
    {
        string text = SteamGridDbSettings.Clean(_readClipboard());
        if (text.Length == 0)
        {
            ShowStatus("Nada copiado ainda. Copie a chave no SteamGridDB e toque em Colar de novo.", error: true);
            return;
        }

        ApiKey = text;
        ShowStatus("Chave colada.", error: false);
    }

    [RelayCommand]
    private async Task TestAsync()
    {
        if (IsTesting)
        {
            return;
        }

        var key = Build() with { Enabled = true };
        if (key.Problem() is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        IsTesting = true;
        ShowStatus("Falando com o SteamGridDB…", error: false);
        try
        {
            await _test(key).ConfigureAwait(true);
            ShowStatus(Enabled ? "Tudo certo: o SteamGridDB respondeu. Salve para usar." : "Tudo certo: o SteamGridDB respondeu. Ligue \"Usar o SteamGridDB\" e salve.", error: false);
        }
        catch (SteamGridDbException ex)
        {
            ShowStatus(SteamGridDbErrors.Message(ex.Failure), error: true);
        }
        finally
        {
            IsTesting = false;
        }
    }

    [RelayCommand]
    private void Save()
    {
        var settings = Build();
        if (settings.Enabled && settings.Problem() is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        _save(settings);
    }

    [RelayCommand]
    private void Cancel() => _cancel();

    /// <summary>Configuração montada dos campos (a chave salva vale quando o campo fica vazio).</summary>
    public SteamGridDbSettings Build()
    {
        string key = SteamGridDbSettings.Clean(ApiKey);
        return new SteamGridDbSettings(Enabled, key.Length > 0 ? key : _saved.ApiKey) { AutoFill = AutoFill, Animated = _saved.Animated };
    }

    private void ShowStatus(string text, bool error)
    {
        Status = text;
        StatusIsError = error;
    }
}
