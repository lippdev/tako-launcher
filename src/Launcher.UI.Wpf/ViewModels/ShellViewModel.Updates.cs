using System.Net.Http;
using System.Text.Json;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Updates;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Atualização automática: ao abrir, procura e baixa em segundo plano; pronta, pergunta se reinicia agora ou instala ao
/// sair. Durante um jogo nada aparece: o aviso espera o jogo terminar.
/// </summary>
public sealed partial class ShellViewModel
{
    private static readonly TimeSpan StartupDelay = TimeSpan.FromSeconds(5);
    private UpdateService? _updates;
    private UpdateInfo? _readyUpdate;
    private string? _readyFile;
    private bool _promptPending;
    private string? _readyHighlight;

    [ObservableProperty]
    private string _updateStatus = $"Versão {UpdateService.CurrentVersion} · {UpdateService.InstallKind}";

    [ObservableProperty]
    private bool _isUpdating;

    public UpdatePreferences UpdatePreferences { get; } = UpdatePreferences.Load();

    /// <summary>Atualização baixada que ainda não foi aplicada: instalada ao fechar o Tako.</summary>
    public string? PendingUpdateFile => _readyFile;

    public void StartAutoUpdate(UpdateService updates)
    {
        _updates = updates;
        if (!UpdateService.IsPublishedBuild)
        {
            UpdateStatus = $"Versão {UpdateService.CurrentVersion} · atualizações automáticas só nas versões instalada e portátil";
            return;
        }

        if (UpdatePreferences.AutoUpdate)
        {
            _ = CheckForUpdatesAsync(manual: false, startupDelay: StartupDelay);
        }
    }

    [RelayCommand]
    private Task CheckUpdatesNow() => CheckForUpdatesAsync(manual: true, startupDelay: TimeSpan.Zero);

    [RelayCommand]
    private void ToggleAutoUpdate()
    {
        UpdatePreferences.AutoUpdate = !UpdatePreferences.AutoUpdate;
        UpdatePreferences.Save();
        OnPropertyChanged(nameof(UpdatePreferences));
    }

    [RelayCommand]
    private void TogglePrereleaseUpdates()
    {
        UpdatePreferences.Prereleases = !UpdatePreferences.Prereleases;
        UpdatePreferences.Save();
        OnPropertyChanged(nameof(UpdatePreferences));
        if (UpdatePreferences.Prereleases)
        {
            Toast("Versões de teste (alpha/beta) podem ter falhas; desligar não volta para a versão anterior");
        }
    }

    private async Task CheckForUpdatesAsync(bool manual, TimeSpan startupDelay)
    {
        if (_updates is null || IsUpdating)
        {
            return;
        }

        if (!UpdateService.IsPublishedBuild)
        {
            Toast("Atualizações automáticas só funcionam nas versões instalada e portátil");
            return;
        }

        if (_readyFile is not null)
        {
            ShowUpdateReady();
            return;
        }

        IsUpdating = true;
        try
        {
            await Task.Delay(startupDelay).ConfigureAwait(true); // deixa a abertura terminar antes de usar a rede
            UpdateStatus = "Procurando atualizações…";
            var update = await _updates.CheckAsync(UpdatePreferences.Prereleases).ConfigureAwait(true);
            if (update is null)
            {
                UpdateStatus = $"Versão {UpdateService.CurrentVersion} · {UpdateService.InstallKind} · você está na versão mais recente";
                if (manual)
                {
                    Toast("O Tako está atualizado");
                }

                return;
            }

            if (!update.CanInstall)
            {
                UpdateStatus = $"Tako {update.Version} disponível (baixe pela página da release)";
                Toast($"Tako {update.Version} disponível · abra a página da release em Configurações");
                return;
            }

            UpdateStatus = $"Baixando Tako {update.Version}…";
            var progress = new Progress<double>(p => UpdateStatus = $"Baixando Tako {update.Version}… {p:P0}");
            _readyFile = await _updates.DownloadAsync(update, progress).ConfigureAwait(true);
            _readyHighlight = await _updates.PortugueseHighlightAsync(update).ConfigureAwait(true) ?? ReleaseFeed.FirstNote(update.Notes);
            _readyUpdate = update;
            UpdateStatus = $"Tako {update.Version} baixada · instala ao reiniciar";
            ShowUpdateReady();
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException or TimeoutException or JsonException or System.IO.IOException or InvalidOperationException)
        {
            UpdateStatus = $"Versão {UpdateService.CurrentVersion} · {UpdateService.InstallKind} · não foi possível procurar atualizações";
            if (manual)
            {
                Toast($"Não foi possível atualizar: {ex.Message}");
            }
        }
        finally
        {
            IsUpdating = false;
        }
    }

    /// <summary>Aviso de atualização pronta; durante o jogo, espera ele terminar (ver <see cref="OnOverlaysChangedForUpdates"/>).</summary>
    private void ShowUpdateReady()
    {
        if (_readyUpdate is null || _readyFile is null)
        {
            return;
        }

        if (RunningGame is not null || TopOverlay is not null)
        {
            _promptPending = true;
            return;
        }

        _promptPending = false;
        var update = _readyUpdate;
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"Tako {update.Version} pronta para instalar{(update.IsPrerelease ? " (versão de teste)" : string.Empty)}",
            _readyHighlight ?? "Nova versão baixada e verificada.",
            [
                new OverlayAction("Reiniciar e atualizar", ActionStyle.Primary, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    ApplyUpdate(relaunch: true);
                }), IsDefault: true),
                new OverlayAction("Depois (instala ao sair)", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
            ]);
        PushOverlay(dialog);
    }

    /// <summary>Chamado quando as sobreposições mudam: mostra o aviso que esperava o jogo terminar.</summary>
    private void OnOverlaysChangedForUpdates()
    {
        if (_promptPending && RunningGame is null && TopOverlay is null)
        {
            ShowUpdateReady();
        }
    }

    private void ApplyUpdate(bool relaunch)
    {
        if (_updates is null || _readyFile is null)
        {
            return;
        }

        if (RunningGame is not null)
        {
            Toast("Encerre o jogo antes de atualizar");
            return;
        }

        try
        {
            _updates.Apply(_readyFile, relaunch);
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or System.IO.IOException or InvalidOperationException)
        {
            Toast($"Não foi possível iniciar a atualização: {ex.Message}");
            return;
        }

        _readyFile = null; // não aplica de novo ao sair
        System.Windows.Application.Current?.Shutdown();
    }

    /// <summary>Ao fechar o Tako com uma atualização baixada: instala sem reabrir.</summary>
    public void ApplyPendingUpdateOnExit()
    {
        if (_readyFile is not null && _updates is not null)
        {
            try
            {
                _updates.Apply(_readyFile, relaunch: false);
            }
            catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or System.IO.IOException or InvalidOperationException)
            {
                // Sem atualizar desta vez; o arquivo verificado continua em %TEMP% e é reaproveitado na próxima abertura.
            }

            _readyFile = null;
        }
    }
}
