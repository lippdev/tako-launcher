using System.Globalization;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library.Metadata.ScreenScraper;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Dados de ROMs (ScreenScraper), ADR-0022, no mesmo padrão da IGDB: cada usuário usa a própria conta
/// de screenscraper.fr. Passo a passo curto, campos com "Colar" (para quem usa só o controle), "Testar" mostrando o nível
/// da conta e a cota do dia, e "Salvar". A senha salva nunca volta para a tela. Sem as credenciais do software (versões
/// que não vieram do build de release), mostra que o ScreenScraper ainda não está disponível, mas deixa salvar a conta.
/// </summary>
public sealed partial class ScreenScraperSettingsViewModel : ObservableObject, IOverlayViewModel
{
    public const string SignUpUrl = "https://www.screenscraper.fr/membreinscription.php";

    private readonly ScreenScraperSettings _saved;
    private readonly Func<ScreenScraperSettings, Task<ScreenScraperAccount>> _test;
    private readonly Action<ScreenScraperSettings> _save;
    private readonly Action _openSite;
    private readonly Func<string?> _readClipboard;
    private readonly Action _cancel;

    /// <summary>O segredo aparece como ● até o usuário pedir para ver (ninguém no sofá lê a senha na TV).</summary>
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(SecretToggleLabel))]
    private bool _isSecretVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(EnabledLabel))]
    private bool _enabled;

    [ObservableProperty]
    private string _user;

    [ObservableProperty]
    private string _password = string.Empty;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(TestLabel))]
    private bool _isTesting;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    private bool _statusIsError;

    public ScreenScraperSettingsViewModel(ScreenScraperSettings saved, bool hasSoftwareAccess, Func<ScreenScraperSettings, Task<ScreenScraperAccount>> test, Action<ScreenScraperSettings> save, Action openSite, Func<string?> readClipboard, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(saved);
        _saved = saved;
        HasSoftwareAccess = hasSoftwareAccess;
        _test = test;
        _save = save;
        _openSite = openSite;
        _readClipboard = readClipboard;
        _cancel = cancel;
        _enabled = saved.Enabled;
        _user = saved.User;
        if (!hasSoftwareAccess)
        {
            ShowStatus(ScreenScraperErrors.NoSoftwareAccess, error: true);
        }
    }

    /// <summary>Esta versão tem as credenciais do software; sem elas, o aviso fica no topo.</summary>
    public bool HasSoftwareAccess { get; }

    public string EnabledLabel => Enabled ? "Buscar dados de ROMs no ScreenScraper: sim" : "Buscar dados de ROMs no ScreenScraper: não";

    public string TestLabel => IsTesting ? "Testando…" : "Testar";

    public string PasswordLabel => _saved.Password.Length > 0 ? "Senha (já salva; deixe vazio para manter)" : "Senha";

    public IReadOnlyList<string> Steps { get; } =
    [
        "1. Crie uma conta gratuita em screenscraper.fr (botão abaixo) e confirme pelo link que chega no seu e-mail.",
        "2. Escreva o seu usuário e a sua senha do ScreenScraper nos campos (ou copie e toque em Colar).",
        "3. Toque em Testar: o Tako mostra o nível da sua conta e quantas consultas ainda restam hoje. Depois, Salvar.",
    ];

    public bool HasStatus => Status is not null;

    public string SecretToggleLabel => IsSecretVisible ? "Ocultar" : "Mostrar";

    [RelayCommand]
    private void ToggleSecretVisible() => IsSecretVisible = !IsSecretVisible;

    [RelayCommand]
    private void ToggleEnabled() => Enabled = !Enabled;

    [RelayCommand]
    private void OpenSite()
    {
        _openSite();
        ShowStatus("Abrimos o ScreenScraper no navegador. Crie a conta, confirme o e-mail e volte aqui.", error: false);
    }

    [RelayCommand]
    private void PasteUser()
    {
        if (Paste("usuário") is { } text)
        {
            User = text.Trim();
        }
    }

    [RelayCommand]
    private void PastePassword()
    {
        if (Paste("senha") is { } text)
        {
            // Senha pode ter espaços no meio; só as quebras de linha da cópia saem.
            Password = text.Trim('\r', '\n');
        }
    }

    [RelayCommand]
    private async Task TestAsync()
    {
        if (IsTesting)
        {
            return;
        }

        var account = Build() with { Enabled = true };
        if (account.Problem() is { } invalid)
        {
            ShowStatus(invalid, error: true);
            return;
        }

        if (!HasSoftwareAccess)
        {
            ShowStatus(ScreenScraperErrors.NoSoftwareAccess, error: true);
            return;
        }

        IsTesting = true;
        ShowStatus("Falando com o ScreenScraper…", error: false);
        try
        {
            var info = await _test(account).ConfigureAwait(true);
            ShowStatus(Describe(info) + (Enabled ? " Salve para buscar os dados das ROMs." : " Ligue \"Buscar dados de ROMs\" e salve."), error: false);
        }
        catch (ScreenScraperException ex)
        {
            ShowStatus(ScreenScraperErrors.Message(ex.Failure), error: true);
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

    /// <summary>"Tudo certo: conta nível 1 · 19.980 de 20.000 consultas restam hoje · 1 consulta por vez."</summary>
    public static string Describe(ScreenScraperAccount account)
    {
        ArgumentNullException.ThrowIfNull(account);
        var pt = CultureInfo.GetCultureInfo("pt-BR");
        string quota = account.MaxRequestsPerDay > 0
            ? $"{account.RemainingToday.ToString("N0", pt)} de {account.MaxRequestsPerDay.ToString("N0", pt)} consultas restam hoje"
            : "cota do dia não informada";
        string threads = account.MaxThreads == 1 ? "1 consulta por vez" : $"{account.MaxThreads.ToString(pt)} consultas ao mesmo tempo";
        return $"Tudo certo: conta {account.User}, nível {account.Level.ToString(pt)} · {quota} · {threads}.";
    }

    /// <summary>Conta montada dos campos, com a senha salva se o campo ficou vazio.</summary>
    public ScreenScraperSettings Build() =>
        new(Enabled, User.Trim(), Password.Length > 0 ? Password : _saved.Password);

    private string? Paste(string field)
    {
        string text = _readClipboard() ?? string.Empty;
        if (text.Trim().Length == 0)
        {
            ShowStatus($"Nada copiado ainda. Copie o seu {field} e toque em Colar de novo.", error: true);
            return null;
        }

        ShowStatus(field == "senha" ? "Senha colada." : "Usuário colado.", error: false);
        return text;
    }

    private void ShowStatus(string text, bool error)
    {
        Status = text;
        StatusIsError = error;
    }
}
