using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library.Metadata.Igdb;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Dados dos jogos (IGDB), ADR-0021: cada usuário usa a própria chave da Twitch. Passo a passo curto na
/// tela, campos para colar o Client ID e o Client Secret (teclado físico ou virtual), "Testar" com mensagem clara e
/// "Salvar". O segredo salvo nunca volta para a tela: o campo vazio mantém o que já está salvo.
/// </summary>
public sealed partial class IgdbSettingsViewModel : ObservableObject, IOverlayViewModel
{
    public const string ConsoleUrl = "https://dev.twitch.tv/console/apps/create";

    private readonly IgdbSettings _saved;
    private readonly Func<IgdbSettings, Task> _test;
    private readonly Action<IgdbSettings> _save;
    private readonly Action _openConsole;
    private readonly Func<string?> _readClipboard;
    private readonly Action _cancel;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(EnabledLabel))]
    private bool _enabled;

    [ObservableProperty]
    private string _clientId;

    [ObservableProperty]
    private string _clientSecret = string.Empty;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(TestLabel))]
    private bool _isTesting;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    private bool _statusIsError;

    public IgdbSettingsViewModel(IgdbSettings saved, Func<IgdbSettings, Task> test, Action<IgdbSettings> save, Action openConsole, Func<string?> readClipboard, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(saved);
        _saved = saved;
        _test = test;
        _save = save;
        _openConsole = openConsole;
        _readClipboard = readClipboard;
        _cancel = cancel;
        _enabled = saved.Enabled;
        _clientId = saved.ClientId;
    }

    public string EnabledLabel => Enabled ? "Buscar dados na IGDB: sim" : "Buscar dados na IGDB: não";

    public string TestLabel => IsTesting ? "Testando…" : "Testar";

    /// <summary>O segredo salvo não aparece; o rótulo diz que ele existe.</summary>
    public string SecretLabel => _saved.ClientSecret.Length > 0 ? "Client Secret (já salvo; deixe vazio para manter)" : "Client Secret";

    public IReadOnlyList<string> Steps { get; } =
    [
        "1. Entre em dev.twitch.tv/console com a sua conta da Twitch (a verificação em duas etapas precisa estar ligada).",
        "2. \"Registrar seu aplicativo\": qualquer nome (ex.: Tako de Fulano), URL de redirecionamento OAuth http://localhost e categoria \"Application Integration\". Tipo de cliente: Confidencial.",
        "3. Em \"Gerenciar\", copie o Client ID e toque em Colar ao lado do campo (ou digite).",
        "4. Toque em \"Novo segredo\", copie o Client Secret e cole no outro campo. Depois, Testar e Salvar.",
    ];

    public bool HasStatus => Status is not null;

    [RelayCommand]
    private void ToggleEnabled() => Enabled = !Enabled;

    [RelayCommand]
    private void OpenConsole()
    {
        _openConsole();
        ShowStatus("Abrimos o site da Twitch no navegador. Siga os passos e volte aqui para colar a chave.", error: false);
    }

    /// <summary>"Colar" ao lado do campo: com o controle, copiar no navegador e colar aqui sem teclado.</summary>
    [RelayCommand]
    private void PasteClientId()
    {
        if (Paste("Client ID") is { } text)
        {
            ClientId = text;
        }
    }

    [RelayCommand]
    private void PasteSecret()
    {
        if (Paste("Client Secret") is { } text)
        {
            ClientSecret = text;
        }
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
        ShowStatus("Falando com a Twitch e a IGDB…", error: false);
        try
        {
            await _test(key).ConfigureAwait(true);
            ShowStatus(Enabled ? "Tudo certo: a IGDB respondeu. Salve para buscar os dados dos jogos." : "Tudo certo: a IGDB respondeu. Ligue \"Buscar dados na IGDB\" e salve.", error: false);
        }
        catch (IgdbException ex)
        {
            ShowStatus(IgdbErrors.Message(ex.Failure), error: true);
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

    /// <summary>Configuração montada dos campos: sem espaços (texto colado costuma trazer) e com o segredo salvo se o campo ficou vazio.</summary>
    public IgdbSettings Build()
    {
        string id = IgdbSettings.Clean(ClientId);
        string secret = IgdbSettings.Clean(ClientSecret);
        return new IgdbSettings(Enabled, id, secret.Length > 0 ? secret : _saved.ClientSecret);
    }

    private string? Paste(string field)
    {
        string text = IgdbSettings.Clean(_readClipboard());
        if (text.Length == 0)
        {
            ShowStatus($"Nada copiado ainda. Copie o {field} no site da Twitch e toque em Colar de novo.", error: true);
            return null;
        }

        ShowStatus($"{field} colado.", error: false);
        return text;
    }

    private void ShowStatus(string text, bool error)
    {
        Status = text;
        StatusIsError = error;
    }
}
