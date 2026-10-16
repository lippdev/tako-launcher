using System.IO;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using Launcher.Core.Library;
using Launcher.Core.Persistence;
using Launcher.Core.Presentation;
using Launcher.Core.Session;
using Launcher.UI.Wpf.Services;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Coleções (UX-003), configurações por jogo (UX-007) e o menu rápido do jogo (UX-005). Tudo pelo controle: nomes e
/// argumentos no teclado virtual, escolhas em diálogos e no painel lateral.
/// </summary>
public sealed partial class ShellViewModel
{
    private readonly GameSettingsStore _gameSettings;

    /// <summary>Coleções do usuário (UX-003).</summary>
    public CollectionLibrary Collections { get; }

    // ---------------------------------------------------------------- Configurações por jogo (UX-007)

    /// <summary>Configurações do Tako que a sessão usa, num só lugar.</summary>
    public GlobalSessionSettings GlobalSessionSettings() => new(
        Settings.GameScreenEnabled,
        Settings.GameScreenId,
        Settings.GameScreenHideOthers,
        DisplayMode.TryParse(Settings.GameScreenMode),
        Settings.GameScreenHdr,
        Settings.GameAudioOutput,
        SessionTools.SavedFpsLimit,
        SessionTools.FpsCounterEnabled);

    /// <summary>O que vale na sessão deste jogo: o único ponto onde a sessão lê as configurações (ver <see cref="GameSettingsResolver"/>).</summary>
    public EffectiveSessionSettings ResolveSessionSettings(string gameId) =>
        GameSettingsResolver.Resolve(GlobalSessionSettings(), LoadGameSettings(gameId));

    public GameSettingsOverride LoadGameSettings(string gameId)
    {
        try
        {
            return _gameSettings.Load(gameId);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogGameSettingsFailed(_logger, ex, "ler", gameId);
            return GameSettingsOverride.None; // sem banco legível o jogo abre com as configurações do Tako
        }
    }

    private static SessionToolChoices ToolChoices(EffectiveSessionSettings effective) =>
        new(effective.FpsLimit, effective.FpsLimitFromGame, effective.ShowFps, effective.ShowFpsFromGame);

    /// <summary>Painel "Configurações deste jogo".</summary>
    public void OpenGameSettings(GameItemViewModel item)
    {
        var context = new GameSettingsContext(
            GlobalSessionSettings(),
            () => Settings.GameScreenId.Length == 0 ? [] : GameScreen.ListModes(Settings.GameScreenId),
            GameAudioOutputs,
            SessionTools.DisplayRefreshHz,
            saved => SaveGameSettings(item.Id, saved),
            (current, done) => EditText("Argumentos extras", "Ex.: -dx11 -windowed", current ?? string.Empty, text =>
                text.Length == 0 ? "Vazio: o jogo abre só com os argumentos dele · ☰ conclui" : "☰ ou Pronto salva · B cancela", text =>
                {
                    done(text);
                    return true;
                }),
            Toast);
        PushOverlay(new GameSettingsViewModel(item.Title, LoadGameSettings(item.Id), context));
    }

    private void SaveGameSettings(string gameId, GameSettingsOverride settings)
    {
        try
        {
            _gameSettings.Save(gameId, settings);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogGameSettingsFailed(_logger, ex, "salvar", gameId);
            Toast("Não foi possível salvar as configurações deste jogo · veja os logs");
        }
    }

    private IReadOnlyList<AudioOutputChoice> GameAudioOutputs()
    {
        try
        {
            return AudioVolume.CanChangeOutput ? [.. AudioVolume.Outputs().Select(d => new AudioOutputChoice(d.Id, d.Name))] : [];
        }
        catch (RuntimeEnvironmentException)
        {
            return [];
        }
    }

    // ---------------------------------------------------------------- Menu rápido do jogo (UX-005)

    /// <summary>☰ (ou M) numa capa: as ações mais usadas sem abrir os detalhes.</summary>
    public void ShowQuickActions(GameItemViewModel item)
    {
        var game = item.Game;
        DialogViewModel? dialog = null;
        void Run(Action action)
        {
            CloseOverlay(dialog!);
            action();
        }

        var actions = new List<OverlayAction>();
        if (game.CanPlay || item.IsRunning)
        {
            actions.Add(new OverlayAction(item.IsRunning ? "Voltar ao jogo" : "Jogar", ActionStyle.Primary, new RelayCommand(() => Run(() => Launch(item))), IsDefault: true));
        }

        actions.Add(new OverlayAction(item.IsFavorite ? "Tirar dos favoritos" : "Favoritar", ActionStyle.Secondary, new RelayCommand(() => Run(() => ToggleFavorite(item)))));
        actions.Add(new OverlayAction("Adicionar à coleção", ActionStyle.Secondary, new RelayCommand(() => Run(() => ChooseCollections(item)))));
        actions.Add(new OverlayAction("Configurações deste jogo", ActionStyle.Secondary, new RelayCommand(() => Run(() => OpenGameSettings(item)))));
        actions.Add(new OverlayAction("Trocar capa", ActionStyle.Secondary, new RelayCommand(() => Run(() => OpenArtPicker(item)))));
        actions.Add(new OverlayAction("Fontes e IDs", ActionStyle.Secondary, new RelayCommand(() => Run(() => OpenSourceIds(item)))));
        actions.Add(new OverlayAction("Detalhes", ActionStyle.Secondary, new RelayCommand(() => Run(() => OpenDetails(item))), IsDefault: actions.Count == 0));
        dialog = new DialogViewModel(item.Title, QuickActionsText(item), actions);
        PushOverlay(dialog);
    }

    private string QuickActionsText(GameItemViewModel item)
    {
        var names = Collections.Of(item.Id).Select(c => c.Name).ToList();
        int changes = LoadGameSettings(item.Id).ChangedCount;
        string collections = names.Count == 0 ? "Em nenhuma coleção" : $"Coleções: {string.Join(", ", names)}";
        string settings = changes == 0 ? "configurações do Tako" : $"{changes} {(changes == 1 ? "configuração própria" : "configurações próprias")}";
        return $"{item.StoreName} · {collections} · {settings}";
    }

    // ---------------------------------------------------------------- Coleções (UX-003)

    /// <summary>Marca/desmarca o jogo nas coleções; "Nova coleção" cria uma já com ele.</summary>
    public void ChooseCollections(GameItemViewModel item, string? focusId = null)
    {
        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        var actions = Collections.Collections
            .Select(c => new OverlayAction(
                c.Contains(item.Id) ? $"{c.Name} ✓" : c.Name,
                c.Contains(item.Id) ? ActionStyle.Primary : ActionStyle.Secondary,
                new RelayCommand(() =>
                {
                    Close();
                    bool added = Toggled(() => Collections.Toggle(c.Id, item.Id));
                    Toast(added ? $"{item.Title} entrou em {c.Name}" : $"{item.Title} saiu de {c.Name}");
                    ChooseCollections(item, c.Id); // continua no diálogo para marcar outras
                }),
                IsDefault: c.Id == focusId))
            .ToList();
        actions.Add(new OverlayAction("Nova coleção", ActionStyle.Secondary, new RelayCommand(() =>
        {
            Close();
            NewCollection(item);
        }), IsDefault: actions.Count == 0));
        actions.Add(new OverlayAction("Pronto", ActionStyle.Secondary, new RelayCommand(Close)));
        dialog = new DialogViewModel(
            $"Coleções de {item.Title}",
            actions.Count == 2 ? "Você ainda não tem coleções. Crie uma para juntar jogos do seu jeito." : "Escolha para pôr ou tirar o jogo (✓ = está na coleção).",
            actions);
        PushOverlay(dialog);
    }

    /// <summary>Configurações › Coleções: criar, renomear e excluir.</summary>
    public void ManageCollections()
    {
        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        var actions = Collections.Collections
            .Select(c => new OverlayAction($"{c.Name} · {LibraryText.GameCount(CountExisting(c))}", ActionStyle.Secondary, new RelayCommand(() =>
            {
                Close();
                CollectionActions(c.Id);
            })))
            .ToList();
        actions.Add(new OverlayAction("Nova coleção", ActionStyle.Primary, new RelayCommand(() =>
        {
            Close();
            NewCollection(null);
        }), IsDefault: true));
        actions.Add(new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(Close)));
        dialog = new DialogViewModel(
            "Coleções",
            Collections.Collections.Count == 0
                ? "Junte jogos do seu jeito (ex.: \"Para jogar com amigos\"). Elas aparecem na Início e nos filtros."
                : "Escolha uma coleção para renomear ou excluir. Para pôr jogos nela, use ☰ numa capa ou Coleções nos detalhes.",
            actions);
        PushOverlay(dialog);
    }

    /// <summary>"Ver todos" de uma fileira: a biblioteca inteira filtrada pela coleção.</summary>
    public void OpenCollection(string collectionId)
    {
        _libraryCriteria = _libraryCriteria.ClearFilters().ToggleCollection(collectionId);
        OpenLibrary(LibraryFilter.All);
    }

    private int CountExisting(CollectionInfo collection) => Library.Games.Count(g => collection.Contains(g.Id));

    private void CollectionActions(string id)
    {
        if (Collections.Find(id) is not { } collection)
        {
            return;
        }

        DialogViewModel? dialog = null;
        void Close() => CloseOverlay(dialog!);
        dialog = new DialogViewModel(
            collection.Name,
            $"{LibraryText.GameCount(CountExisting(collection))} nesta coleção.",
            [
                new OverlayAction("Renomear", ActionStyle.Primary, new RelayCommand(() =>
                {
                    Close();
                    EditText("Renomear coleção", "Nome da coleção", collection.Name, text => NameHint(text, id), text => TryCollection(() => Collections.Rename(id, text), $"Coleção renomeada para {CollectionNames.Clean(text)}"));
                }), IsDefault: true),
                new OverlayAction("Excluir", ActionStyle.Danger, new RelayCommand(() =>
                {
                    Close();
                    ConfirmDeleteCollection(collection);
                })),
                new OverlayAction("Voltar", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    Close();
                    ManageCollections();
                })),
            ]);
        PushOverlay(dialog);
    }

    private void ConfirmDeleteCollection(CollectionInfo collection)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"Excluir {collection.Name}?",
            "Só a coleção é excluída; os jogos continuam na biblioteca.",
            [
                new OverlayAction("Excluir", ActionStyle.Danger, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    if (Change(() => Collections.Delete(collection.Id)))
                    {
                        Toast($"Coleção {collection.Name} excluída");
                    }
                })),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true),
            ]);
        PushOverlay(dialog);
    }

    /// <summary>Nome pelo teclado virtual; com <paramref name="item"/>, a coleção já nasce com o jogo.</summary>
    private void NewCollection(GameItemViewModel? item) =>
        EditText("Nova coleção", "Nome da coleção", string.Empty, text => NameHint(text, null), text =>
            TryCollection(
                () => Collections.Create(text, item?.Id),
                item is null ? $"Coleção {CollectionNames.Clean(text)} criada" : $"{item.Title} entrou na nova coleção {CollectionNames.Clean(text)}"));

    private string NameHint(string text, string? exceptId) =>
        text.Length == 0 ? $"Até {CollectionNames.MaxLength} letras · ☰ ou Pronto salva · B cancela"
        : CollectionNames.Problem(text, Collections.Collections, exceptId) ?? "☰ ou Pronto salva · B cancela";

    /// <summary>Aplica a mudança de coleção; nome que não serve fica no teclado com a explicação.</summary>
    private bool TryCollection(Action change, string done)
    {
        try
        {
            if (!Change(change))
            {
                return true; // falha de disco: já avisou; fecha o teclado
            }

            Toast(done);
            return true;
        }
        catch (ArgumentException ex)
        {
            Toast(ex.Message.Split(" (Parameter", 2)[0]);
            return false;
        }
    }

    /// <summary>Muda as coleções; o disco pode falhar (avisa e segue). Devolve falso nesse caso.</summary>
    private bool Change(Action change)
    {
        try
        {
            change();
            return true;
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogCollectionsSaveFailed(_logger, ex);
            Toast("Não foi possível salvar a coleção · veja os logs");
            return false;
        }
    }

    private bool Toggled(Func<bool> change)
    {
        bool result = false;
        return Change(() => { result = change(); }) && result;
    }

    /// <summary>
    /// Teclado virtual para um texto. <paramref name="save"/> devolve verdadeiro para fechar; falso mantém o teclado
    /// aberto (ex.: nome repetido), com a explicação na linha de baixo.
    /// </summary>
    private void EditText(string title, string placeholder, string text, Func<string, string> describe, Func<string, bool> save)
    {
        SearchViewModel? entry = null;
        entry = SearchViewModel.ForText(title, placeholder, text, describe, typed =>
        {
            if (save(typed))
            {
                CloseOverlay(entry!);
            }
        });
        PushOverlay(entry);
    }

    private void OnCollectionsChanged()
    {
        Collections.ApplyTo(Library.Games);
        _libraryCriteria = _libraryCriteria.KeepCollections(Collections.Collections.Select(c => c.Id)); // coleção excluída sai do filtro
        foreach (var item in _items.Values)
        {
            item.Refresh();
        }

        CurrentPage.Refresh();
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível {Action} as configurações do jogo {GameId}")]
    private static partial void LogGameSettingsFailed(ILogger logger, Exception exception, string action, string gameId);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível salvar as coleções")]
    private static partial void LogCollectionsSaveFailed(ILogger logger, Exception exception);
}
