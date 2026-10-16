using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Input;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Uma linha de "Fontes e IDs": ação da fonte (digitar o id, nenhum, automático) ou um candidato da busca.</summary>
public sealed partial class SourceRowViewModel(string key, string label, string? detail, ICommand command, ArtService art, Uri? thumb) : LazyThumb(art, thumb, 64), INavKeyed
{
    public string Key { get; } = key;

    public string Label { get; } = label;

    public string? Detail { get; } = detail;

    public ICommand Command { get; } = command;

    /// <summary>Candidato (com miniatura) ou ação.</summary>
    public bool IsCandidate { get; init; }

    /// <summary>O casado agora (✓).</summary>
    public bool IsCurrent { get; init; }

    /// <summary>O mais provável pelo nome (★).</summary>
    public bool IsBest { get; init; }

    string INavKeyed.NavKey => Key;
}

/// <summary>Uma fonte em "Fontes e IDs": o id em uso, o estado da busca e as linhas (ações e candidatos).</summary>
public sealed partial class SourceSectionViewModel(string source) : ObservableObject
{
    [ObservableProperty]
    private string _summary = string.Empty;

    [ObservableProperty]
    private string? _status;

    public string Source { get; } = source;

    public ObservableCollection<SourceRowViewModel> Rows { get; } = [];

    /// <summary>Resultados da última busca (antes dos filtros de plataforma e ano).</summary>
    public IReadOnlyList<MetadataMatch> Results { get; set; } = [];

    /// <summary>O mais provável pelo nome (para "Aplicar o mais provável em todas").</summary>
    public MetadataMatch? Best { get; set; }
}

/// <summary>
/// "Fontes e IDs" (ADR-0023): modal com cada fonte ligada, o id casado e o nome/ano que ele representa. Ao abrir, busca o
/// nome do jogo em todas as fontes ao mesmo tempo (cada uma no próprio ritmo) e lista os candidatos com miniatura, ano e
/// plataforma; ✓ marca o casado e ★ o mais provável. Escolher um candidato, colar o id (ou o endereço da página) ou
/// "Nenhum" fixa a fonte e busca de novo os dados e as imagens. "Automático" solta. O termo é editável (teclado
/// virtual) e há filtros de plataforma e ano.
/// </summary>
public sealed partial class SourceIdsViewModel : ObservableObject, IOverlayViewModel, IOverlayInput, IOverlayHints
{
    private readonly ISourceIdService _service;
    private readonly ArtService _art;
    private readonly SourceIdsHost _host;
    private string? _platformFilter;
    private int? _yearFilter;

    [ObservableProperty]
    private string _term;

    [ObservableProperty]
    private bool _isSearching;

    public SourceIdsViewModel(GameItemViewModel item, ISourceIdService service, ArtService art, SourceIdsHost host)
    {
        ArgumentNullException.ThrowIfNull(item);
        Item = item;
        _service = service;
        _art = art;
        _host = host;
        _term = item.Title;
        Sections = [.. service.SourceStates(item.Game).Select(s => new SourceSectionViewModel(s.Source))];
        Tools =
        [
            new ModalTab("ids:term", TermLabel, new RelayCommand(EditTerm)),
            new ModalTab("ids:search", "Buscar de novo", new RelayCommand(() => _ = SearchAllAsync())),
            new ModalTab("ids:best", "Aplicar o mais provável em todas", new AsyncRelayCommand(ApplyBestAsync)),
            new ModalTab("ids:platform", "Plataforma: todas", new RelayCommand(CyclePlatform)),
            new ModalTab("ids:year", "Ano: qualquer", new RelayCommand(CycleYear)),
        ];
        Tools[0].IsSelected = true;
        RefreshSections();
        _ = SearchAllAsync();
    }

    public GameItemViewModel Item { get; }

    public string Title => $"Fontes e IDs de {Item.Title}";

    public IReadOnlyList<ModalTab> Tools { get; }

    public IReadOnlyList<SourceSectionViewModel> Sections { get; }

    public bool HasSections => Sections.Count > 0;

    private string TermLabel => $"Nome buscado: {Term}";

    public IReadOnlyList<(string Buttons, string Label, AppCommand Command)> Hints =>
    [
        ("a", "Escolher", AppCommand.Accept),
        ("b", "Fechar", AppCommand.Back),
        ("y", "Outro nome", AppCommand.Search),
        ("x", "Buscar de novo", AppCommand.Favorite),
    ];

    public bool HandleCommand(AppCommand command)
    {
        switch (command)
        {
            case AppCommand.Search:
                EditTerm();
                return true;
            case AppCommand.Favorite:
                _ = SearchAllAsync();
                return true;
            default:
                return false;
        }
    }

    /// <summary>Busca o termo em todas as fontes ao mesmo tempo; cada seção mostra os resultados quando chegam.</summary>
    public async Task SearchAllAsync()
    {
        IsSearching = true;
        var tasks = Sections.Select(SearchSectionAsync).ToList();
        await Task.WhenAll(tasks).ConfigureAwait(true);
        IsSearching = false;
    }

    private async Task SearchSectionAsync(SourceSectionViewModel section)
    {
        section.Status = $"Buscando \"{Term}\"…";
        try
        {
            section.Results = await _service.SearchSourceAsync(Item.Game, section.Source, Term, CancellationToken.None).ConfigureAwait(true);
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or TaskCanceledException or System.Text.Json.JsonException)
        {
            section.Results = [];
        }

        section.Best = section.Results.FirstOrDefault(r => r.IsExact) ?? TitleMatcher.Best(Term, section.Results);
        section.Status = section.Results.Count == 0 ? $"Nada encontrado para \"{Term}\"." : null;
        Fill(section);
    }

    private void RefreshSections()
    {
        foreach (var section in Sections)
        {
            Fill(section);
        }
    }

    private void Fill(SourceSectionViewModel section)
    {
        var state = _service.SourceStates(Item.Game).FirstOrDefault(s => s.Source == section.Source);
        section.Summary = Describe(state);
        section.Rows.Clear();
        string source = section.Source;
        section.Rows.Add(new SourceRowViewModel($"ids:{source}:type", "Digitar ou colar o ID", SourceIdInput.Example(source), new RelayCommand(() => EnterId(source)), _art, null));
        section.Rows.Add(new SourceRowViewModel($"ids:{source}:none", "Nenhum", "Não usar esta fonte para este jogo", new AsyncRelayCommand(() => PinAsync(source, string.Empty, "nenhum")), _art, null));
        if (state?.IsPinned == true)
        {
            section.Rows.Add(new SourceRowViewModel($"ids:{source}:auto", "Automático", "Soltar a escolha; o Tako casa sozinho de novo", new AsyncRelayCommand(() => PinAsync(source, null, "automático")), _art, null));
        }

        foreach (var match in section.Results.Where(Visible))
        {
            var chosen = match;
            string detail = string.Join(" · ", new[] { match.Year?.ToString(CultureInfo.InvariantCulture), match.Platforms, $"ID {match.Id}" }.Where(p => !string.IsNullOrEmpty(p)));
            section.Rows.Add(new SourceRowViewModel($"ids:{source}:{match.Id}", (match.Id == state?.Id ? "✓ " : match == section.Best ? "★ " : string.Empty) + match.Title, detail,
                new AsyncRelayCommand(() => PinAsync(source, chosen.Id, chosen.Title)), _art, match.CoverUrl)
            {
                IsCandidate = true,
                IsCurrent = match.Id == state?.Id,
                IsBest = match == section.Best,
            });
        }
    }

    private bool Visible(MetadataMatch match) =>
        (_platformFilter is null || match.Platforms?.Contains(_platformFilter, StringComparison.OrdinalIgnoreCase) == true)
        && (_yearFilter is null || match.Year == _yearFilter);

    private static string Describe(SourceState? state) => state switch
    {
        null => "Sem dados",
        { IsNone: true } => "Nenhum · escolhido por você",
        { Id: null } => "Sem correspondência",
        _ => $"ID {state.Id}" + (state.Title is null ? string.Empty : $" · {state.Title}") + (state.Year is int year ? $" ({year})" : string.Empty) + (state.IsPinned ? " · escolhido por você" : " · automático"),
    };

    private void EditTerm() => _host.EditText("Buscar pelo nome", "Nome do jogo", Term, text =>
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return;
        }

        Term = text.Trim();
        Tools[0].Label = TermLabel;
        _ = SearchAllAsync();
    });

    /// <summary>Digitar ou colar: valida na fonte e mostra o nome antes de aplicar.</summary>
    private void EnterId(string source) => _host.EnterId(source, async text =>
    {
        var input = SourceIdInput.Parse(source, text);
        if (!input.IsValid)
        {
            _host.Toast(input.Problem!);
            return;
        }

        _host.Toast($"Conferindo no {source}…");
        var found = await _service.ValidateSourceAsync(source, text, CancellationToken.None).ConfigureAwait(true);
        if (found is not { } valid)
        {
            _host.Toast($"O {source} não conhece esse ID. Confira o número ou use a busca.");
            return;
        }

        string name = valid.Metadata.Title ?? valid.Id;
        string year = valid.Metadata.ReleaseDate is { } date ? $" ({date.Year})" : string.Empty;
        _host.Confirm($"Usar {name}{year}?", $"{source} · ID {valid.Id}. Os dados e as imagens desta fonte vêm deste jogo a partir de agora.", () => _ = PinAsync(source, valid.Id, name));
    });

    private async Task PinAsync(string source, string? id, string name)
    {
        var section = Sections.First(s => s.Source == source);
        section.Status = "Aplicando e buscando os dados…";
        bool ok = await _service.PinSourceAsync(Item.Game, source, id).ConfigureAwait(true);
        section.Status = null;
        Fill(section);
        _host.Toast(ok
            ? id is null ? $"{source} volta a casar sozinho" : id.Length == 0 ? $"{source} não será usado para {Item.Title}" : $"{Item.Title} usa {name} no {source}"
            : "Não foi possível salvar agora; veja os registros");
        _host.Focus($"ids:{source}:type");
        _host.Changed();
    }

    private async Task ApplyBestAsync()
    {
        var best = Sections.Where(s => s.Best is not null).ToList();
        if (best.Count == 0)
        {
            _host.Toast("Nenhuma fonte achou um resultado provável; tente outro nome");
            return;
        }

        foreach (var section in best)
        {
            await PinAsync(section.Source, section.Best!.Id, section.Best.Title).ConfigureAwait(true);
        }

        _host.Toast($"Aplicado o mais provável em {best.Count} {(best.Count == 1 ? "fonte" : "fontes")}");
    }

    private void CyclePlatform()
    {
        var platforms = Sections.SelectMany(s => s.Results).SelectMany(r => (r.Platforms ?? string.Empty).Split(", ", StringSplitOptions.RemoveEmptyEntries)).Distinct(StringComparer.OrdinalIgnoreCase).Order(StringComparer.Ordinal).ToList();
        int index = _platformFilter is null ? -1 : platforms.IndexOf(_platformFilter);
        _platformFilter = index + 1 < platforms.Count ? platforms[index + 1] : null;
        Tools[3].Label = $"Plataforma: {_platformFilter ?? "todas"}";
        RefreshSections();
    }

    private void CycleYear()
    {
        var years = Sections.SelectMany(s => s.Results).Select(r => r.Year).OfType<int>().Distinct().Order().ToList();
        int index = _yearFilter is int y ? years.IndexOf(y) : -1;
        _yearFilter = index + 1 < years.Count ? years[index + 1] : null;
        Tools[4].Label = $"Ano: {(_yearFilter is int year ? year.ToString(CultureInfo.InvariantCulture) : "qualquer")}";
        RefreshSections();
    }
}

/// <summary>O que "Fontes e IDs" pede à Shell.</summary>
public sealed record SourceIdsHost(
    Action<string, string, string, Action<string>> EditText,
    Action<string, Func<string, Task>> EnterId,
    Action<string, string, Action> Confirm,
    Action<string> Toast,
    Action<string?> Focus,
    Action Changed);
