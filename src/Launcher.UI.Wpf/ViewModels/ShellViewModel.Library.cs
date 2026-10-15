using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Biblioteca real: abrir jogos, adicionar e remover jogos, opções por jogo e metadados (MVP-023/024).</summary>
public sealed partial class ShellViewModel
{
    private const string ImageFilter = "Imagens|*.jpg;*.jpeg;*.png;*.webp;*.bmp|Todos os arquivos|*.*";

    /// <summary>Busca dados e artes dos jogos em segundo plano (cache primeiro) e de novo quando a biblioteca muda.</summary>
    public void StartMetadataUpdates()
    {
        _ = _metadata.RefreshAsync(() => Library.Games);
        Library.Changed += (_, _) => _ = _metadata.RefreshAsync(() => Library.Games);
    }

    /// <summary>Um item por jogo da biblioteca; jogos recriados (ex.: capa trocada) ganham item novo.</summary>
    private void SyncItems()
    {
        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (var game in Library.Games)
        {
            ids.Add(game.Id);
            if (!_items.TryGetValue(game.Id, out var item) || item.Game != game)
            {
                item = new GameItemViewModel(game, _art, _time);
                _items[game.Id] = item;
                // Jogo recriado enquanto roda (ex.: editado): o item novo continua "Voltar ao jogo".
                if (RunningGame?.Id == game.Id)
                {
                    item.IsRunning = true;
                    RunningGame = item;
                }

                _ = item.LoadCoverAsync();
            }
        }

        foreach (string stale in _items.Keys.Where(id => !ids.Contains(id)).ToList())
        {
            _items.Remove(stale);
        }
    }

    private void OnMetadataUpdated(string id)
    {
        if (Library.Find(id) is not { } game || !_items.TryGetValue(id, out var item) || item.Game != game)
        {
            return;
        }

        var previousCover = game.CoverUri;
        _metadata.ApplyCached(game);
        item.Refresh();
        if (game.CoverUri != previousCover)
        {
            _ = item.ReloadArtAsync();
            if (_backdropGameId == id)
            {
                SetBackdrop(item);
            }
        }

        CurrentPage.Refresh();
    }

    // ---------------------------------------------------------------- Adicionar, remover, opções

    [RelayCommand]
    public void AddGame()
    {
        var dialog = new OpenFileDialog
        {
            Title = "Adicionar jogo",
            Filter = "Jogos e atalhos|*.exe;*.lnk;*.url;*.bat;*.cmd|Todos os arquivos|*.*",
            CheckFileExists = true,
        };
        if (dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) != true)
        {
            return;
        }

        string path = dialog.FileName;
        if (Library.Games.FirstOrDefault(g => string.Equals(g.LaunchTarget, path, StringComparison.OrdinalIgnoreCase)) is { } existing)
        {
            Toast($"{existing.Title} já está na biblioteca");
            OpenDetails(ItemFor(existing));
            return;
        }

        string title = GuessTitle(path);
        var game = Library.AddManual(ManualGameRecord.Create(title, path, _time.GetUtcNow()));
        Toast($"{title} adicionado · buscando capa e informações");
        OpenDetails(ItemFor(game));
    }

    [RelayCommand]
    public void RescanLibrary()
    {
        Library.Refresh();
        int steam = Library.Games.Count(g => g.Store == GameStore.Steam);
        Toast(steam == 0 ? "Nenhum jogo da Steam encontrado neste PC" : $"{steam} jogos da Steam encontrados");
    }

    [RelayCommand]
    public void RefreshMetadata()
    {
        _ = _metadata.RefreshAsync(() => Library.Games, force: true);
        Toast($"Atualizando dados dos jogos ({_metadata.ProviderName}) em segundo plano");
    }

    /// <summary>"Opções" nos detalhes: ações conforme a origem do jogo.</summary>
    public void ShowGameOptions(GameItemViewModel item)
    {
        var game = item.Game;
        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        var actions = new List<OverlayAction>();
        if (game.Store == GameStore.Steam && game.SteamAppId is int appId)
        {
            actions.Add(new OverlayAction("Abrir na Steam", ActionStyle.Primary, new RelayCommand(() => { Close(); OpenUri($"steam://nav/games/details/{appId}"); }), IsDefault: true));
        }

        if (game.Store == GameStore.Manual)
        {
            actions.Add(new OverlayAction("Corrigir correspondência", ActionStyle.Primary, new RelayCommand(() => { Close(); _ = FixMatchAsync(item); }), IsDefault: true));
            actions.Add(new OverlayAction("Trocar capa", ActionStyle.Secondary, new RelayCommand(() => { Close(); ChangeCover(item); })));
        }

        if (game.InstallPath is { } folder && Directory.Exists(folder))
        {
            actions.Add(new OverlayAction("Abrir pasta", ActionStyle.Secondary, new RelayCommand(() => { Close(); OpenUri(folder); })));
        }

        if (game.Store == GameStore.Manual)
        {
            actions.Add(new OverlayAction("Remover da biblioteca", ActionStyle.Danger, new RelayCommand(() => { Close(); ConfirmRemove(item); })));
        }

        actions.Add(new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(Close), IsDefault: actions.Count == 0));
        string text = game.Store == GameStore.Manual
            ? $"{game.LaunchTarget}\nDados e artes: {SourceText(game)}"
            : $"{GameTextStore(game)} · dados e artes: {SourceText(game)}";
        dialog = new DialogViewModel(game.Title, text, actions);
        PushOverlay(dialog);
    }

    private string SourceText(Game game) => game.SteamAppId is int id ? $"{_metadata.ProviderName} (appid {id})" : "nenhuma correspondência ainda";

    private static string GameTextStore(Game game) => Core.Presentation.GameText.StoreName(game.Store);

    /// <summary>Como no Plex: mostra os resultados da busca pelo título e fixa o escolhido.</summary>
    private async Task FixMatchAsync(GameItemViewModel item)
    {
        Toast($"Buscando \"{item.Title}\" em {_metadata.ProviderName}…");
        var results = await _metadata.SearchAsync(item.Title).ConfigureAwait(true);
        DialogViewModel? dialog = null;
        var actions = results.Take(5)
            .Select((match, i) => new OverlayAction(match.Title, i == 0 ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(async () =>
            {
                CloseOverlay(dialog!);
                bool ok = await _metadata.PinAsync(item.Game, match.SteamAppId).ConfigureAwait(true);
                Toast(ok ? $"{item.Title} agora usa os dados de {match.Title}" : "Não foi possível buscar os dados agora; tente de novo");
            }), IsDefault: i == 0))
            .Append(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: results.Count == 0))
            .ToList();
        dialog = new DialogViewModel(
            "Qual destes é o seu jogo?",
            results.Count == 0 ? $"Nada encontrado para \"{item.Title}\". Renomeie o atalho ou escolha uma capa manualmente." : $"Resultados para \"{item.Title}\":",
            actions);
        PushOverlay(dialog);
    }

    private void ChangeCover(GameItemViewModel item)
    {
        if (Library.FindManual(item.Id) is not { } record)
        {
            return;
        }

        var dialog = new OpenFileDialog { Title = $"Capa de {item.Title} (vertical, 2:3)", Filter = ImageFilter, CheckFileExists = true };
        if (dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) != true)
        {
            return;
        }

        Library.UpdateManual(record with { CoverPath = dialog.FileName });
        Toast("Capa atualizada");
        if (Library.Find(item.Id) is { } updated && CurrentPage is DetailsViewModel)
        {
            Navigate(new DetailsViewModel(this, ItemFor(updated)), replace: true);
        }
    }

    private void ConfirmRemove(GameItemViewModel item)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"Remover {item.Title}?",
            "Sai só da biblioteca do Tako; nenhum arquivo do jogo é apagado.",
            [
                new OverlayAction("Remover", ActionStyle.Danger, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    Library.RemoveManual(item.Id);
                    Toast($"{item.Title} removido da biblioteca");
                    GoHome();
                })),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true),
            ]);
        PushOverlay(dialog);
    }

    private void OpenUri(string target)
    {
        try
        {
            Process.Start(new ProcessStartInfo(target) { UseShellExecute = true })?.Dispose();
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            Toast($"Não foi possível abrir: {ex.Message}");
        }
    }

    /// <summary>Nome do produto do executável ("ELDEN RING™") ou nome do arquivo sem extensão e separadores.</summary>
    private static string GuessTitle(string path)
    {
        if (string.Equals(Path.GetExtension(path), ".exe", StringComparison.OrdinalIgnoreCase))
        {
            var info = FileVersionInfo.GetVersionInfo(path);
            foreach (string? candidate in (string?[])[info.ProductName, info.FileDescription])
            {
                if (!string.IsNullOrWhiteSpace(candidate) && !candidate.Contains("launcher", StringComparison.OrdinalIgnoreCase) && candidate.Length > 2)
                {
                    return candidate.Replace("™", string.Empty, StringComparison.Ordinal).Replace("®", string.Empty, StringComparison.Ordinal).Trim();
                }
            }
        }

        return Path.GetFileNameWithoutExtension(path).Replace('_', ' ').Replace('-', ' ').Trim();
    }
}
