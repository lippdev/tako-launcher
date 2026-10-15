using System.IO;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Configurações › Temas: importar <c>.pthm</c>, prévia, diagnóstico, remover e restaurar o padrão (MVP-014/015/020).</summary>
public sealed partial class ShellViewModel
{
    private const int MaxDiagnosticLines = 8;
    private readonly Dictionary<string, string> _themeLoadSummaries = new(StringComparer.OrdinalIgnoreCase);

    [RelayCommand]
    public void OpenThemes() => Navigate(new ThemesViewModel(this, _playniteThemes, _themeLoadSummaries), replace: CurrentPage is ThemesViewModel);

    /// <summary>Escolhe um <c>.pthm</c> e instala (o seletor de arquivo é a única parte que não roda no teste de fumaça).</summary>
    public void ImportTheme()
    {
        var dialog = new OpenFileDialog
        {
            Title = "Importar tema do Playnite",
            Filter = $"Temas do Playnite (*{ThemePackage.Extension})|*{ThemePackage.Extension}|Todos os arquivos|*.*",
            CheckFileExists = true,
        };
        if (dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) == true)
        {
            ImportThemeFrom(dialog.FileName);
        }
    }

    /// <summary>Valida e instala o pacote; mostra o resultado (com os motivos da recusa, se houver).</summary>
    public ThemeInstallResult ImportThemeFrom(string packagePath)
    {
        var result = _playniteThemes.Import(packagePath);
        string package = Path.GetFileName(packagePath);
        LogThemeImport(_logger, package, result.Succeeded, result.Diagnostics.Count);
        RefreshThemesPage();
        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        if (result.Succeeded)
        {
            var theme = _playniteThemes.Imported().FirstOrDefault(t => string.Equals(t.Directory, result.Directory, StringComparison.OrdinalIgnoreCase));
            string text = $"{result.FileCount} arquivos instalados em {result.Directory}." + DiagnosticLines(result.Warnings.Select(d => d.ToString()));
            dialog = new DialogViewModel(
                $"Tema \"{result.Manifest!.DisplayName}\" importado",
                text,
                theme is null
                    ? [new OverlayAction("Fechar", ActionStyle.Primary, new RelayCommand(Close), IsDefault: true)]
                    : [
                        new OverlayAction("Abrir prévia", ActionStyle.Primary, new RelayCommand(() => { Close(); ShowThemePreview(theme); }), IsDefault: true),
                        new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(Close)),
                    ]);
        }
        else
        {
            dialog = new DialogViewModel(
                "O tema não foi importado",
                $"{package} foi recusado. Nada foi instalado." + DiagnosticLines(result.Diagnostics.Select(d => d.ToString())),
                [new OverlayAction("Fechar", ActionStyle.Primary, new RelayCommand(Close), IsDefault: true)]);
        }

        PushOverlay(dialog);
        return result;
    }

    /// <summary>Ações de um tema da lista (a lista em si fica simples para navegar com o controle).</summary>
    public void ShowThemeOptions(ThemeItemViewModel item)
    {
        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        var actions = new List<OverlayAction>();
        string text;
        if (item.Theme is not { } theme)
        {
            text = "A interface e o visual do Tako.";
            if (!item.IsSelected)
            {
                actions.Add(new OverlayAction("Usar este tema", ActionStyle.Primary, new RelayCommand(() => { Close(); SelectTheme(item); }), IsDefault: true));
            }

            actions.Add(new OverlayAction("Recarregar tema", item.IsSelected ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() => { Close(); ReloadTakoTheme(); }), IsDefault: item.IsSelected));
        }
        else
        {
            text = $"{theme.Directory}\n{item.Detail}\n\"Usar este tema\" aplica o tema à tela inicial do Tako; Menu (☰ / M) sempre abre as Configurações do Tako."
                + (IsBlockedBySafeMode(theme) ? "\nModo seguro ativo: prévia e diagnóstico ficam indisponíveis até voltar ao normal." : string.Empty);
            if (theme.Manifest.IsCompatible)
            {
                actions.Add(new OverlayAction("Abrir prévia", ActionStyle.Primary, new RelayCommand(() => { Close(); ShowThemePreview(theme); }), IsDefault: true));
                if (!item.IsSelected)
                {
                    actions.Add(new OverlayAction("Usar este tema", ActionStyle.Secondary, new RelayCommand(() => { Close(); SelectTheme(item); })));
                }
            }

            actions.Add(new OverlayAction("Diagnóstico", ActionStyle.Secondary, new RelayCommand(() => { Close(); ShowThemeDiagnostics(item); }), IsDefault: !theme.Manifest.IsCompatible));
            if (item.CanRemove)
            {
                actions.Add(new OverlayAction("Remover", ActionStyle.Danger, new RelayCommand(() => { Close(); ConfirmRemoveTheme(item); })));
            }
        }

        actions.Add(new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(Close)));
        dialog = new DialogViewModel(item.Name, text, actions);
        PushOverlay(dialog);
    }

    /// <summary>Carrega o tema pelo adaptador P10 e mostra X de Y arquivos e os motivos de recusa.</summary>
    public void ShowThemeDiagnostics(ThemeItemViewModel item)
    {
        if (item.Theme is not { } theme || TryLoadTheme(theme) is not { } report)
        {
            return;
        }

        item.LoadSummary = ThemeItemViewModel.Summarize(report);
        var lines = theme.Manifest.Diagnostics.Select(d => d.ToString())
            .Concat(report.Rejected.Select(r => $"Recusado: {r.File} — {r.Reason}"));
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"Diagnóstico: {item.Name}",
            $"{item.LoadSummary} pelo carregador XAML restrito (ThemeApiVersion {theme.Manifest.Manifest.ThemeApiVersionText ?? "?"}; suportado {ThemeManifestValidator.SupportedRange})." + DiagnosticLines(lines),
            [new OverlayAction("Fechar", ActionStyle.Primary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true)]);
        PushOverlay(dialog);
    }

    /// <summary>Prévia de qualquer tema P10 (embutido ou importado) com a biblioteca do launcher.</summary>
    public void ShowThemePreview(InstalledTheme theme)
    {
        if (TryLoadTheme(theme) is not { } report)
        {
            return;
        }

        PlaynitePreviewViewModel? preview = null;
        preview = new PlaynitePreviewViewModel(report, LibraryQuery.RecentlyPlayed(Library.Games).Select(ItemFor), new RelayCommand(() => CloseOverlay(preview!)));
        PushOverlay(preview);
    }

    /// <summary>Volta à interface e ao tema padrão do Tako; a prévia volta ao tema Fullscreen padrão do Playnite.</summary>
    [RelayCommand]
    public void RestoreDefaultTheme()
    {
        var result = _themes.Apply(ThemeService.DefaultTheme);
        TryChangeSelection(null);
        ClosePlayniteUi();
        RefreshThemesPage();
        Toast(result.Succeeded ? "Tema padrão restaurado" : "Tema padrão embutido em uso");
    }

    private void ReloadTakoTheme()
    {
        var result = _themes.Apply(ThemeService.DefaultTheme);
        Toast(result.Succeeded ? "Tema do Tako recarregado pelo carregador XAML restrito" : $"Falha ao recarregar: {result.Error}");
    }

    /// <summary>"Usar este tema": tema Playnite na tela inicial (MVP-018/019) ou de volta à interface do Tako.</summary>
    public void SelectTheme(ThemeItemViewModel item)
    {
        ArgumentNullException.ThrowIfNull(item);
        if (item.Theme is { } theme && IsBlockedBySafeMode(theme))
        {
            Toast("Modo seguro: temas importados não são aplicados. Volte ao normal em Configurações");
            return;
        }

        if (!TryChangeSelection(item.Theme))
        {
            return;
        }

        ApplyPlayniteUi();
        RefreshThemesPage();
        if (item.Theme is null)
        {
            Toast("Interface do Tako em uso");
        }
        else if (PlayniteUi is not null)
        {
            Toast($"{item.Name} aplicado à tela inicial · ☰ abre as Configurações do Tako");
        }
        else if (IsSafeMode)
        {
            Toast($"{item.Name} será aplicado ao sair do modo seguro");
        }
    }

    private void ConfirmRemoveTheme(ThemeItemViewModel item)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"Remover {item.Name}?",
            "Apaga a pasta do tema importado. Se ele estiver em uso, o Tako volta à interface padrão.",
            [
                new OverlayAction("Remover", ActionStyle.Danger, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    RemoveTheme(item);
                })),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true),
            ]);
        PushOverlay(dialog);
    }

    public void RemoveTheme(ThemeItemViewModel item)
    {
        if (!item.CanRemove || item.Theme is null)
        {
            return;
        }

        try
        {
            _playniteThemes.Remove(item.Theme);
            _themeLoadSummaries.Remove(item.Theme.Directory);
            if (PlayniteUi is not null && _playniteThemes.SelectedForUi() is null)
            {
                ClosePlayniteUi();
            }

            Toast($"{item.Name} removido");
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidOperationException)
        {
            Toast($"Não foi possível remover: {ex.Message}");
        }

        RefreshThemesPage();
        RequestFocus("themes:import");
    }

    /// <summary>Tema da "Prévia de tema Playnite 10": o escolhido, ou o embutido no modo seguro.</summary>
    private P10ThemeReport LoadPreviewTheme() =>
        IsSafeMode ? _playniteThemes.LoadDirectory(_playniteThemes.BuiltIn().Directory) : _playniteThemes.LoadSelected();

    /// <summary>Temas importados vêm do disco do usuário: o modo seguro (frente D) não os carrega.</summary>
    private bool IsBlockedBySafeMode(InstalledTheme theme) =>
        IsSafeMode && !string.Equals(theme.Directory, _playniteThemes.BuiltIn().Directory, StringComparison.OrdinalIgnoreCase);

    private P10ThemeReport? TryLoadTheme(InstalledTheme theme)
    {
        if (IsBlockedBySafeMode(theme))
        {
            Toast("Modo seguro: temas importados não são carregados. Volte ao normal em Configurações");
            return null;
        }

        try
        {
            var report = _playniteThemes.LoadDirectory(theme.Directory);
            LogPlaynitePreview(_logger, report.Loaded.Count, report.Rejected.Count);
            _themeLoadSummaries[theme.Directory] = ThemeItemViewModel.Summarize(report);
            return report;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            Toast($"Não foi possível ler o tema: {ex.Message}");
            return null;
        }
    }

    private bool TryChangeSelection(InstalledTheme? theme)
    {
        try
        {
            _playniteThemes.Select(theme);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            Toast($"Não foi possível salvar a escolha do tema: {ex.Message}");
            return false;
        }
    }

    private void RefreshThemesPage()
    {
        if (CurrentPage is ThemesViewModel page)
        {
            page.Refresh();
        }
    }

    private static string DiagnosticLines(IEnumerable<string> lines)
    {
        var list = lines.ToList();
        if (list.Count == 0)
        {
            return string.Empty;
        }

        var shown = list.Take(MaxDiagnosticLines).Select(l => "\n• " + l);
        string more = list.Count > MaxDiagnosticLines ? $"\n… e mais {list.Count - MaxDiagnosticLines}" : string.Empty;
        return "\n" + string.Concat(shown) + more;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Importação de tema {Package}: sucesso={Succeeded}, {Diagnostics} diagnósticos")]
    private static partial void LogThemeImport(ILogger logger, string package, bool succeeded, int diagnostics);
}
