using System.IO;
using System.Security.Cryptography;
using System.Text;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.SteamGridDb;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Capas e metadados (ADR-0023): seletor de imagens por fonte, "Fontes e IDs", SteamGridDB, cache de imagens e capas
/// animadas (só a capa em foco e a arte dos detalhes se mexem; tudo para durante o jogo, com a janela em segundo plano
/// ou com "Reduzir animações").
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Mais capas animando ao mesmo tempo no modo "Sempre" (memória).</summary>
    private const int MaxAnimating = 8;

    private readonly List<GameItemViewModel> _animating = [];
    private readonly List<GameItemViewModel> _recentlyShown = [];
    private readonly IArtCatalog _artCatalog;
    private readonly ISourceIdService _sourceIds;
    private readonly AnimatedArtService _animatedArt;
    private bool _isWindowActive = true;
    private long? _artCacheBytes;

    /// <summary>A janela do Tako está em primeiro plano (fora dela, as capas animadas param).</summary>
    public bool IsWindowActive
    {
        get => _isWindowActive;
        set
        {
            if (_isWindowActive != value)
            {
                _isWindowActive = value;
                UpdateAnimations();
            }
        }
    }

    /// <summary>"Desligado", "Ligado · sua chave", "Ligado · falta a chave" ou "Com problema · chave recusada".</summary>
    public string SteamGridDbSummary => (_metadata.SteamGridDbSettings, _metadata.SteamGridDbFailure) switch
    {
        ({ IsActive: true }, SteamGridDbFailure.Unauthorized) => "Com problema · chave recusada",
        ({ IsActive: true }, SteamGridDbFailure.Network) => "Com problema · sem conexão",
        ({ IsActive: true }, _) => "Ligado · sua chave",
        ({ Enabled: true }, _) => "Ligado · falta a chave",
        _ => "Desligado",
    };

    /// <summary>Configurações das fontes de metadados (para as linhas das Configurações).</summary>
    public MetadataService MetadataSettings => _metadata;

    /// <summary>Tamanho atual do cache de imagens ("12,4 MB"), lido sob demanda.</summary>
    public string ArtCacheSummary => ByteSize.Format(_artCacheBytes ??= _art.DiskCacheBytes());

    // ---------------------------------------------------------------- Seletor de imagens

    /// <summary>Detalhes › Opções › "Trocar imagens" e ☰ › "Trocar capa": o modal com abas de tipo e de fonte.</summary>
    public void OpenArtPicker(GameItemViewModel item, ArtKind kind = ArtKind.Cover)
    {
        ArgumentNullException.ThrowIfNull(item);
        ArtPickerViewModel? picker = null;
        var host = new ArtPickerHost(
            candidate => ApplyArtAsync(item, candidate),
            chosenKind => ResetArt(item, chosenKind),
            (title, placeholder, text, done) => EditText(title, placeholder, text, _ => "☰ ou Pronto busca · B cancela", typed =>
            {
                done(typed);
                return true;
            }),
            (title, matches, chosen) => Choose(title, matches, chosen),
            PickImageFile,
            Toast,
            RequestFocus,
            UpdateHints,
            () => AnimationPolicy.ShouldAnimate(Settings.AnimatedCovers, true, IsInGame, IsWindowActive, Settings.ReducedMotion),
            () => CloseOverlay(picker!))
        {
            ShowAnimated = _metadata.SteamGridDbSettings.Animated,
        };
        picker = new ArtPickerViewModel(item, kind, _artCatalog, _art, _animatedArt, host);
        PushOverlay(picker);
    }

    private async Task<bool> ApplyArtAsync(GameItemViewModel item, ArtCandidate candidate)
    {
        var url = candidate.Url;
        if (candidate.Source == ArtSources.File && url.IsFile)
        {
            // Cópia fora do cache: apagar ou mover o arquivo original (ou limpar o cache) não some com a capa.
            url = await Task.Run(() => CopyChosenFile(url.LocalPath)).ConfigureAwait(true) is { } copy ? new Uri(copy) : url;
        }

        if (!_metadata.SetPinnedArt(item.Game, candidate.Kind, url))
        {
            Toast("Não foi possível salvar a escolha agora; veja os registros");
            return false;
        }

        OnArtChanged(item, candidate.Kind);
        Toast($"{ArtKinds.Label(candidate.Kind)} de {item.Title} aplicada · {candidate.Source}");
        return true;
    }

    private void ResetArt(GameItemViewModel item, ArtKind kind)
    {
        if (_metadata.SetPinnedArt(item.Game, kind, null))
        {
            OnArtChanged(item, kind);
            Toast($"{ArtKinds.Label(kind)} de {item.Title} voltou a ser a automática");
        }
    }

    private void OnArtChanged(GameItemViewModel item, ArtKind kind)
    {
        if (kind == ArtKind.Cover)
        {
            item.ReloadArt();
        }

        if (kind == ArtKind.Hero || _backdropGameId == item.Id)
        {
            RefreshBackdrop();
        }

        item.Refresh();
        CurrentPage.Refresh();
        UpdateAnimations();
    }

    private string? CopyChosenFile(string path)
    {
        try
        {
            Directory.CreateDirectory(AppPaths.ChosenArtDirectory);
            string name = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(path + File.GetLastWriteTimeUtc(path).Ticks)))[..24] + Path.GetExtension(path).ToLowerInvariant();
            string target = Path.Combine(AppPaths.ChosenArtDirectory, name);
            File.Copy(path, target, overwrite: true);
            return target;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogArtCopyFailed(_logger, ex, Path.GetFileName(path));
            return null;
        }
    }

    private static string? PickImageFile()
    {
        var dialog = new OpenFileDialog { Title = "Escolha uma imagem", Filter = ImageFilter, CheckFileExists = true };
        return dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) == true ? dialog.FileName : null;
    }

    /// <summary>Lista de resultados para escolher (A escolhe; B cancela).</summary>
    private void Choose(string title, IReadOnlyList<MetadataMatch> matches, Action<MetadataMatch> chosen)
    {
        DialogViewModel? dialog = null;
        var actions = matches.Take(8)
            .Select((match, i) => new OverlayAction(GameText.MatchLabel(match), i == 0 ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                chosen(match);
            }), IsDefault: i == 0))
            .Append(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))))
            .ToList();
        dialog = new DialogViewModel(title, "Escolha o jogo:", actions);
        PushOverlay(dialog);
    }

    // ---------------------------------------------------------------- Fontes e IDs

    /// <summary>Detalhes › Opções › "Fontes e IDs" e ☰: o id de cada fonte, busca pelo nome em todas e fixação.</summary>
    public void OpenSourceIds(GameItemViewModel item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var host = new SourceIdsHost(
            (title, placeholder, text, done) => EditText(title, placeholder, text, _ => "☰ ou Pronto confirma · B cancela", typed =>
            {
                done(typed);
                return true;
            }),
            EnterSourceId,
            Confirm,
            Toast,
            RequestFocus,
            () => OnMetadataUpdated(item.Id));
        PushOverlay(new SourceIdsViewModel(item, _sourceIds, _art, host));
    }

    /// <summary>Digitar no teclado virtual ou colar da área de transferência (copiado no navegador do celular ou do PC).</summary>
    private void EnterSourceId(string source, Func<string, Task> done)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel($"ID no {source}", SourceIdInput.Example(source) + ". Também vale o endereço da página do jogo.",
        [
            new OverlayAction("Digitar", ActionStyle.Primary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                EditText($"ID no {source}", SourceIdInput.Example(source), string.Empty, text => SourceIdInput.Parse(source, text).Problem ?? "☰ ou Pronto confere · B cancela", text =>
                {
                    _ = done(text);
                    return true;
                });
            }), IsDefault: true),
            new OverlayAction("Colar", ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                if (ReadClipboard() is { Length: > 0 } text)
                {
                    _ = done(text);
                }
                else
                {
                    Toast("Nada copiado ainda: copie o ID ou o endereço da página e tente de novo");
                }
            })),
            new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
        ]);
        PushOverlay(dialog);
    }

    private void Confirm(string title, string text, Action yes)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(title, text,
        [
            new OverlayAction("Usar", ActionStyle.Primary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                yes();
            }), IsDefault: true),
            new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
        ]);
        PushOverlay(dialog);
    }

    // ---------------------------------------------------------------- Configurações › Capas e metadados

    [RelayCommand]
    private void OpenSteamGridDbSettings()
    {
        SteamGridDbSettingsViewModel? editor = null;
        editor = new SteamGridDbSettingsViewModel(
            _metadata.SteamGridDbSettings,
            settings => _metadata.TestSteamGridDbAsync(settings, CancellationToken.None),
            saved =>
            {
                CloseOverlay(editor!);
                _metadata.SaveSteamGridDb(saved);
                OnPropertyChanged(nameof(SteamGridDbSummary));
                CurrentPage.Refresh();
                if (saved.IsActive && saved.AutoFill)
                {
                    _ = _metadata.RefreshAsync(() => Library.Games);
                    Toast("SteamGridDB ligado · buscando capas para os jogos sem capa");
                }
                else
                {
                    Toast(saved.IsActive ? "SteamGridDB ligado · escolha capas em Detalhes › Opções" : "SteamGridDB desligado");
                }
            },
            () => OpenUri(SteamGridDbSettingsViewModel.SiteUrl),
            ReadClipboard,
            () => CloseOverlay(editor!));
        PushOverlay(editor);
    }

    /// <summary>"Usar o SteamGridDB quando faltar capa" (desligado por padrão).</summary>
    public void ToggleSteamGridDbAutoFill()
    {
        var current = _metadata.SteamGridDbSettings;
        if (!current.IsActive)
        {
            Toast("Configure o SteamGridDB com a sua chave primeiro");
            OpenSteamGridDbSettingsCommand.Execute(null);
            return;
        }

        _metadata.SaveSteamGridDb(current with { AutoFill = !current.AutoFill });
        if (!current.AutoFill)
        {
            _ = _metadata.RefreshAsync(() => Library.Games);
        }

        CurrentPage.Refresh();
    }

    /// <summary>Mostrar capas animadas do SteamGridDB no seletor.</summary>
    public void ToggleSteamGridDbAnimated()
    {
        var current = _metadata.SteamGridDbSettings;
        _metadata.SaveSteamGridDb(current with { Animated = !current.Animated });
        CurrentPage.Refresh();
    }

    /// <summary>Capas animadas: Desligado › Só na capa em foco › Sempre.</summary>
    public void CycleAnimatedCovers()
    {
        Settings.AnimatedCovers = AnimationPolicy.Next(Settings.AnimatedCovers);
        UpdateAnimations();
        CurrentPage.Refresh();
    }

    /// <summary>"Limpar cache de imagens": as imagens baixam de novo quando aparecem; as escolhas ficam.</summary>
    public void ClearArtCache()
    {
        long freed = _art.ClearDiskCache();
        _artCacheBytes = null;
        foreach (var item in _items.Values)
        {
            item.ReloadArt();
        }

        RefreshBackdrop();
        Toast(freed > 0 ? $"Cache de imagens limpo · {ByteSize.Format(freed)} liberados" : "O cache de imagens já estava vazio");
        CurrentPage.Refresh();
    }

    // ---------------------------------------------------------------- Capas animadas

    /// <summary>
    /// Decide quais capas se mexem agora (<see cref="AnimationPolicy"/>): a focada e a dos detalhes (ou, no modo "Sempre",
    /// também as últimas que apareceram, até <see cref="MaxAnimating"/>); as outras ficam no primeiro quadro.
    /// </summary>
    private void UpdateAnimations()
    {
        var mode = Settings.AnimatedCovers;
        var focused = new List<GameItemViewModel>();
        if (TopOverlay is null && _focusedContext is GameItemViewModel game)
        {
            focused.Add(game);
        }

        if (CurrentPage is DetailsViewModel details && TopOverlay is null)
        {
            focused.Add(details.Game);
        }

        var wanted = focused.Where(_ => AnimationPolicy.ShouldAnimate(mode, true, IsInGame, IsWindowActive, Settings.ReducedMotion)).ToList();
        if (AnimationPolicy.ShouldAnimate(mode, false, IsInGame, IsWindowActive, Settings.ReducedMotion))
        {
            wanted.AddRange(_recentlyShown.Where(i => !wanted.Contains(i)).Take(MaxAnimating - wanted.Count));
        }

        foreach (var item in _animating.Where(i => !wanted.Contains(i)).ToList())
        {
            item.StopAnimation();
            _animating.Remove(item);
        }

        foreach (var item in wanted.Where(i => !_animating.Contains(i)))
        {
            _animating.Add(item);
            _ = item.StartAnimationAsync(_animatedArt);
        }
    }

    /// <summary>Uma capa apareceu na tela: no modo "Sempre", entra na lista das que podem animar.</summary>
    private void OnCoverShown(GameItemViewModel item)
    {
        if (Settings.AnimatedCovers != AnimatedCoversMode.Always || !AnimatedArtService.MightAnimate(item.Game.ShownCover))
        {
            return;
        }

        _recentlyShown.Remove(item);
        _recentlyShown.Insert(0, item);
        if (_recentlyShown.Count > MaxAnimating)
        {
            _recentlyShown.RemoveAt(_recentlyShown.Count - 1);
        }

        // Fora da leitura do binding que mostrou a capa (iniciar a animação avisa a própria capa).
        System.Windows.Threading.Dispatcher.CurrentDispatcher.BeginInvoke(UpdateAnimations);
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível copiar a imagem escolhida {File}")]
    private static partial void LogArtCopyFailed(ILogger logger, Exception exception, string file);
}
