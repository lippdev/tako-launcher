using System.IO;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Playnite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Importar a biblioteca do Playnite (STORE-009, ADR-0019): consentimento explícito, leitura só de leitura, prévia e
/// confirmação. Nada é gravado nos arquivos do Playnite.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Pasta da biblioteca do Playnite; nula = procurar (<see cref="PlayniteLocator"/>). Os testes apontam para uma pasta de exemplo.</summary>
    public string? PlayniteLibraryFolder { get; set; }

    /// <summary>Última prévia lida, à espera da confirmação.</summary>
    public PlayniteImportPlan? PendingPlayniteImport { get; private set; }

    /// <summary>Configurações › Importar do Playnite: primeiro o consentimento, com o que será lido.</summary>
    public void ImportFromPlaynite()
    {
        string? folder = PlayniteLibraryFolder ?? PlayniteLocator.Find(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData));
        if (folder is null || !File.Exists(Path.Combine(folder, "games.db")))
        {
            Toast("Biblioteca do Playnite não encontrada neste PC");
            return;
        }

        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            "Importar do Playnite?",
            $"O Tako vai ler a biblioteca do Playnite em {folder} para trazer jogos, tempo jogado, favoritos e categorias "
            + "(viram coleções). É só leitura: nada muda no Playnite. Jogos das lojas que o Tako já lê não são duplicados. "
            + "Antes de importar, você vê a prévia.",
            [
                new OverlayAction("Ler biblioteca", ActionStyle.Primary, UiCommand(async () =>
                {
                    CloseOverlay(dialog!);
                    await ReadPlayniteLibraryAsync(folder).ConfigureAwait(true);
                }), IsDefault: true),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
            ]);
        PushOverlay(dialog);
    }

    /// <summary>Lê a biblioteca fora da thread da UI e mostra a prévia (ou o motivo de não conseguir).</summary>
    public async Task ReadPlayniteLibraryAsync(string folder)
    {
        PlayniteLibrary playnite;
        try
        {
            playnite = await Task.Run(() => PlayniteLibrary.Read(folder)).ConfigureAwait(true);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidDataException)
        {
            LogPlayniteReadFailed(_logger, ex);
            Toast(ex switch
            {
                FileNotFoundException => "Biblioteca do Playnite não encontrada neste PC",
                InvalidDataException => "Não foi possível ler a biblioteca do Playnite: formato não reconhecido",
                UnauthorizedAccessException => "Sem permissão para ler a biblioteca do Playnite",
                _ => "A biblioteca do Playnite está em uso: feche o Playnite e tente de novo",
            });
            return;
        }

        var plan = PlayniteImport.Plan(playnite, Library.Games, Collections.Collections.Select(c => c.Name), _time.GetUtcNow());
        PendingPlayniteImport = plan;
        ShowPlaynitePreview(plan);
    }

    private void ShowPlaynitePreview(PlayniteImportPlan plan)
    {
        DialogViewModel? dialog = null;
        var cancel = new OverlayAction(plan.IsEmpty ? "Fechar" : "Cancelar", ActionStyle.Secondary, new RelayCommand(() =>
        {
            PendingPlayniteImport = null;
            CloseOverlay(dialog!);
        }), IsDefault: plan.IsEmpty);
        string examples = plan.New.Count == 0
            ? string.Empty
            : "\n\nNovos: " + string.Join(", ", plan.New.Take(5).Select(g => g.Record.Title)) + (plan.New.Count > 5 ? "…" : string.Empty);
        dialog = new DialogViewModel(
            "Prévia da importação",
            plan.Summary + examples,
            plan.IsEmpty
                ? [cancel]
                : [
                    new OverlayAction("Importar", ActionStyle.Primary, new RelayCommand(() =>
                    {
                        CloseOverlay(dialog!);
                        ApplyPlayniteImport(plan);
                    }), IsDefault: true),
                    cancel,
                ]);
        PushOverlay(dialog);
    }

    /// <summary>Grava a importação confirmada: jogos novos, favoritos e tempo, e as coleções (categorias do Playnite).</summary>
    public void ApplyPlayniteImport(PlayniteImportPlan plan)
    {
        ArgumentNullException.ThrowIfNull(plan);
        PendingPlayniteImport = null;
        var state = new Dictionary<string, Func<GameStateRecord?, GameStateRecord>>(StringComparer.Ordinal);
        foreach (var added in plan.New)
        {
            var source = added.Source;
            state[added.Record.Id] = current => PlayniteImport.MergeState(current, source, GameStore.Manual);
        }

        foreach (var match in plan.Matched)
        {
            var source = match.Source;
            var store = Library.Find(match.TakoId)?.Store ?? GameStore.Manual;
            state[match.TakoId] = current => PlayniteImport.MergeState(current, source, store);
        }

        Library.Import([.. plan.New.Select(g => g.Record)], state);

        var memberships = plan.New.Select(g => (Id: g.Record.Id, g.Collections)).Concat(plan.Matched.Select(g => (Id: g.TakoId, g.Collections)));
        foreach (var (id, names) in memberships)
        {
            foreach (string name in names)
            {
                var collection = Collections.Collections.FirstOrDefault(c => LibraryQuery.TitleComparer.Equals(c.Name, name))
                    ?? CreateImportedCollection(name);
                if (collection is not null && !collection.Contains(id))
                {
                    Collections.Toggle(collection.Id, id);
                }
            }
        }

        Collections.ApplyTo(Library.Games);
        LogPlayniteImported(_logger, plan.New.Count, plan.Matched.Count, plan.CollectionNames.Count);
        Toast(plan.New.Count switch
        {
            0 => "Favoritos, tempo e coleções do Playnite importados",
            1 => "1 jogo importado do Playnite",
            int n => $"{n} jogos importados do Playnite",
        });
        CurrentPage.Refresh();
    }

    private CollectionInfo? CreateImportedCollection(string name)
    {
        try
        {
            return Collections.Create(name);
        }
        catch (ArgumentException)
        {
            return null; // nome que o Tako não aceita: o jogo entra, só sem essa coleção
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Playnite: não foi possível ler a biblioteca")]
    private static partial void LogPlayniteReadFailed(ILogger logger, Exception ex);

    [LoggerMessage(Level = LogLevel.Information, Message = "Playnite: {New} jogos novos, {Matched} completados, {Collections} coleções criadas")]
    private static partial void LogPlayniteImported(ILogger logger, int @new, int matched, int collections);
}
