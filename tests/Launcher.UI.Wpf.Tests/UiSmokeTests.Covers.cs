using System.IO;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.SteamGridDb;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0023, só com comandos de controle: o seletor de imagens abre como modal sobre os detalhes, a primeira miniatura
    /// recebe o foco, → mostra a prévia da vizinha, A aplica (a capa fica fixada), LB/RB trocam o tipo, a linha de fontes
    /// troca para o SteamGridDB (X vai aos filtros, A troca o estilo), ☰ volta para a automática e B fecha devolvendo o
    /// foco. "Fontes e IDs" busca em todas as fontes ao abrir, A num candidato fixa o id, "Aplicar o mais provável" fixa
    /// todos. A categoria Capas e metadados reúne as fontes (SteamGridDB com "Testar" simulado), capas animadas e o
    /// cache. Catálogo de imagens e fontes simulados, com imagens geradas no teste: o CI nunca vai à rede.
    /// </summary>
    private static void CoversAndSourceIds(ShellViewModel shell, MainWindow window, MetadataService metadata, FakeArtCatalog catalog, FakeSourceIds sources)
    {
        var nav = window.Navigator;
        var game = shell.Library.AddManual(ManualGameRecord.Create("Lanterna Distante", @"C:\Jogos\Lanterna\lanterna.exe", DateTimeOffset.Now));
        Pump(0.4);
        var item = shell.ItemFor(game);
        shell.OpenDetails(item);
        Pump(0.8);
        var origin = Nav.GetKey(nav.Focused!);

        // Detalhes › Opções tem "Trocar imagens" e "Fontes e IDs" (substituem "Trocar capa" e "Corrigir correspondência").
        shell.ShowGameOptions(item);
        Pump(0.4);
        var options = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Contains(options.Actions, a => a.Label == "Trocar imagens");
        Assert.Contains(options.Actions, a => a.Label == "Fontes e IDs");
        Assert.DoesNotContain(options.Actions, a => a.Label == "Corrigir correspondência");
        options.Actions.First(a => a.Label == "Trocar imagens").Command.Execute(null);
        Pump(1.2);

        var picker = Assert.IsType<ArtPickerViewModel>(shell.TopOverlay);
        Assert.Equal(ArtKind.Cover, picker.Kind);
        Assert.Equal(["Steam", "IGDB", "SteamGridDB", "Arquivo do PC"], picker.Sources.Select(s => s.Label));
        Assert.Equal("Steam", picker.Source);
        Assert.Equal("art:0", Nav.GetKey(nav.Focused!)); // a primeira miniatura já está em foco
        Assert.Contains(shell.Hints, h => h.Label == "Automática");
        Press(window, AppCommand.Right);
        Assert.Equal("art:1", Nav.GetKey(nav.Focused!));
        Pump(0.6);
        Assert.Same(picker.Items[1], picker.Focused);
        Assert.NotNull(picker.Preview);
        Snap(window, "90-seletor-capa");

        Press(window, AppCommand.Accept); // A aplica
        Pump(0.4);
        Assert.Equal(picker.Items[1].Candidate.Url, game.PinnedArt[ArtKind.Cover]);
        Assert.Equal(game.ShownCover, picker.Items[1].Candidate.Url);
        Assert.True(picker.Items[1].IsCurrent);
        Assert.True(picker.IsPinned);

        // LB/RB trocam o tipo; a fonte acompanha o que cada uma oferece.
        Press(window, AppCommand.NextSection);
        Pump(0.8);
        Assert.Equal(ArtKind.Hero, picker.Kind);
        Assert.All(picker.Items, i => Assert.Equal(ArtKind.Hero, i.Candidate.Kind));
        Snap(window, "91-seletor-fundo");
        Press(window, AppCommand.PreviousSection);
        Pump(0.8);
        Assert.Equal(ArtKind.Cover, picker.Kind);

        // SteamGridDB: o jogo casado aparece, X leva aos filtros e A troca o estilo; Y busca outro jogo.
        Assert.True(nav.FocusDefault("art-source:SteamGridDB"));
        Press(window, AppCommand.Accept);
        Pump(1);
        Assert.True(picker.IsSteamGridDb);
        Assert.Contains("Lanterna", picker.SteamGridDbGame, StringComparison.Ordinal);
        Assert.Contains(shell.Hints, h => h.Label == "Filtros");
        var author = picker.Items.First().Candidate.Author;
        Assert.Equal("art:0", Nav.GetKey(nav.Focused!));
        Pump(0.4);
        Assert.Contains(author!, picker.PreviewCredit, StringComparison.Ordinal);
        Press(window, AppCommand.Favorite); // X
        Assert.Equal("art-filter:style", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Pump(0.8);
        Assert.Equal("alternate", picker.Filter.Style);
        Assert.Equal("alternate", catalog.LastQuery!.Filter!.Style);
        Snap(window, "92-seletor-steamgriddb");
        Press(window, AppCommand.Search); // Y: teclado virtual com o nome do jogo
        Pump(0.4);
        Assert.IsType<SearchViewModel>(shell.TopOverlay);
        Press(window, AppCommand.Back);
        Pump(0.3);
        Assert.Same(picker, shell.TopOverlay);

        // ☰ volta para a automática.
        Press(window, AppCommand.Menu);
        Pump(0.4);
        Assert.False(game.PinnedArt.ContainsKey(ArtKind.Cover));
        Assert.False(picker.IsPinned);
        Assert.Contains("automática", shell.ToastMessage, StringComparison.Ordinal);

        // B fecha e o foco volta para onde estava nos detalhes.
        Press(window, AppCommand.Back);
        Pump(0.5);
        Assert.Null(shell.TopOverlay);
        Assert.Equal(origin, Nav.GetKey(nav.Focused!));

        // --- Fontes e IDs: busca em todas as fontes ao abrir; candidatos com ✓ e ★.
        shell.OpenSourceIds(item);
        Pump(1);
        var ids = Assert.IsType<SourceIdsViewModel>(shell.TopOverlay);
        Assert.Equal(["Steam", "IGDB", "SteamGridDB"], ids.Sections.Select(s => s.Source));
        Assert.All(ids.Sections, s => Assert.Contains(s.Rows, r => r.IsCandidate));
        Assert.Contains(ids.Sections.Single(s => s.Source == "IGDB").Rows, r => r.IsBest);
        Assert.Equal(3, sources.Searches.Count); // as três ao mesmo tempo
        Assert.Equal("ids:term", Nav.GetKey(nav.Focused!));
        Snap(window, "93-fontes-e-ids");
        Assert.True(nav.FocusDefault("ids:IGDB:900002"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        Assert.Equal(("IGDB", "900002"), sources.Pinned[^1]);
        Assert.Contains("ID 900002", ids.Sections.Single(s => s.Source == "IGDB").Summary, StringComparison.Ordinal);
        Assert.True(nav.FocusDefault("ids:SteamGridDB:none"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        Assert.Equal(("SteamGridDB", string.Empty), sources.Pinned[^1]);
        Assert.True(nav.FocusDefault("ids:best"));
        Press(window, AppCommand.Accept);
        Pump(1);
        Assert.Contains(sources.Pinned, p => p == ("Steam", "4000001"));
        Snap(window, "94-fontes-e-ids-fixadas");
        Press(window, AppCommand.Back);
        Pump(0.4);
        Assert.Null(shell.TopOverlay);

        // --- Configurações › Capas e metadados.
        shell.OpenSettings();
        Pump(0.8);
        var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        Assert.True(nav.FocusDefault("settings-cat:metadados"));
        Pump(0.4);
        Assert.Equal("metadados", settings.SelectedCategory.Id);
        Assert.Equal(
            ["settings:igdb", "settings:screenscraper", "settings:steamgriddb", "settings:sgdb-autofill", "settings:sgdb-animated", "settings:animated-covers", "settings:art-order", "settings:metadata", "settings:art-cache"],
            settings.SelectedCategory.Rows.Where(r => !r.IsHeader).Select(r => r.Key));
        Assert.Equal("Desligado", shell.SteamGridDbSummary);
        Snap(window, "95-configuracoes-capas-e-metadados");

        metadata.SteamGridDbTester = (key, _) => key.ApiKey.StartsWith("errada", StringComparison.Ordinal)
            ? Task.FromException(new SteamGridDbException(SteamGridDbFailure.Unauthorized, "401"))
            : Task.CompletedTask;
        Assert.True(nav.FocusDefault("settings:steamgriddb"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var editor = Assert.IsType<SteamGridDbSettingsViewModel>(shell.TopOverlay);
        Assert.Equal("sgdb:enabled", Nav.GetKey(nav.Focused!));
        Assert.Equal(3, editor.Steps.Count);
        Press(window, AppCommand.Accept); // liga
        editor.ApiKey = "errada0123456789abcdef0123456789";
        editor.TestCommand.Execute(null);
        Pump(0.3);
        Assert.True(editor.StatusIsError);
        Assert.Contains("Preferências › API", editor.Status, StringComparison.Ordinal);
        editor.ApiKey = " 0123456789abcdef0123456789abcdef\n";
        editor.TestCommand.Execute(null);
        Pump(0.3);
        Assert.False(editor.StatusIsError);
        Snap(window, "96-configuracoes-steamgriddb");
        editor.SaveCommand.Execute(null);
        Pump(0.4);
        Assert.Null(shell.TopOverlay);
        Assert.Equal("Ligado · sua chave", shell.SteamGridDbSummary);
        Assert.False(metadata.SteamGridDbSettings.AutoFill); // desligado por padrão

        Assert.True(nav.FocusDefault("settings:sgdb-autofill"));
        Press(window, AppCommand.Accept);
        Assert.True(metadata.SteamGridDbSettings.AutoFill);
        Assert.Equal(AnimatedCoversMode.FocusedOnly, shell.Settings.AnimatedCovers);
        Assert.True(nav.FocusDefault("settings:animated-covers"));
        Press(window, AppCommand.Accept);
        Assert.Equal(AnimatedCoversMode.Always, shell.Settings.AnimatedCovers);
        Press(window, AppCommand.Accept);
        Press(window, AppCommand.Accept);
        Assert.Equal(AnimatedCoversMode.FocusedOnly, shell.Settings.AnimatedCovers);
        Assert.True(nav.FocusDefault("settings:art-cache"));
        Press(window, AppCommand.Accept);
        Assert.StartsWith("Cache de imagens", shell.ToastMessage!.Replace("O cache", "Cache", StringComparison.Ordinal), StringComparison.Ordinal);
        Snap(window, "97-configuracoes-capas-fim");

        // Desliga de novo para não mudar o resto da jornada.
        metadata.SaveSteamGridDb(SteamGridDbSettings.Off);
        shell.Library.RemoveManual(game.Id);
        shell.GoHome();
        Pump(0.6);
    }

    /// <summary>Imagens de exemplo geradas no teste (cores lisas), servidas como arquivos locais: nada de rede nem arte real.</summary>
    internal sealed class FakeArtCatalog : IArtCatalog
    {
        private readonly string _folder = Directory.CreateDirectory(Path.Combine(Path.GetTempPath(), "tako-smoke-art-" + Guid.NewGuid().ToString("N"))).FullName;

        public ArtQuery? LastQuery { get; private set; }

        public IReadOnlyList<string> Sources(Game game) => [ArtSources.Steam, ArtSources.Igdb, ArtSources.SteamGridDb, ArtSources.File];

        public Task<IReadOnlyList<ArtCandidate>> ListAsync(ArtQuery query, CancellationToken cancellationToken)
        {
            LastQuery = query;
            int count = query.Source == ArtSources.SteamGridDb ? 10 : 4;
            var (width, height) = query.Kind switch
            {
                ArtKind.Hero => (384, 124),
                ArtKind.Logo => (200, 80),
                ArtKind.Icon => (64, 64),
                _ => (120, 180),
            };
            IReadOnlyList<ArtCandidate> list = [.. Enumerable.Range(0, count).Select(i => new ArtCandidate(query.Source, query.Kind, new Uri(Image($"{query.Source}-{query.Kind}-{query.Filter?.Style}-{i}", width, height, i)))
            {
                Label = $"{ArtKinds.Label(query.Kind)} {i + 1}",
                Width = width,
                Height = height,
                Author = query.Source == ArtSources.SteamGridDb ? $"Autora {i + 1}" : null,
                Style = query.Filter?.Style ?? "alternate",
                IsAnimated = query.Source == ArtSources.SteamGridDb && i == 1,
            })];
            return Task.FromResult(list);
        }

        public Task<IReadOnlyList<MetadataMatch>> SearchSteamGridDbAsync(string term, CancellationToken cancellationToken) =>
            Task.FromResult<IReadOnlyList<MetadataMatch>>([new MetadataMatch(ArtSources.SteamGridDb, "7", term)]);

        public Task<MetadataMatch?> SteamGridDbGameAsync(Game game, CancellationToken cancellationToken) =>
            Task.FromResult<MetadataMatch?>(new MetadataMatch(ArtSources.SteamGridDb, "5249", "Lanterna Distante") { Year = 2024 });

        private string Image(string name, int width, int height, int shade)
        {
            string path = Path.Combine(_folder, string.Concat(name.Select(c => char.IsLetterOrDigit(c) || c == '-' ? c : '_')) + ".png");
            if (!File.Exists(path))
            {
                var pixels = new byte[width * height * 4];
                for (int p = 0; p < pixels.Length; p += 4)
                {
                    pixels[p] = (byte)(60 + (shade * 40 % 180));
                    pixels[p + 1] = 90;
                    pixels[p + 2] = (byte)(200 - (shade * 30 % 150));
                    pixels[p + 3] = 255;
                }

                var encoder = new PngBitmapEncoder();
                encoder.Frames.Add(BitmapFrame.Create(BitmapSource.Create(width, height, 96, 96, PixelFormats.Bgra32, null, pixels, width * 4)));
                using var file = File.Create(path);
                encoder.Save(file);
            }

            return path;
        }
    }

    /// <summary>"Fontes e IDs" simulado: três fontes, resultados fixos e as fixações registradas.</summary>
    internal sealed class FakeSourceIds : ISourceIdService
    {
        private readonly Dictionary<string, string?> _ids = new(StringComparer.Ordinal) { ["Steam"] = null, ["IGDB"] = "900001", ["SteamGridDB"] = null };

        public List<string> Searches { get; } = [];

        public List<(string Source, string Id)> Pinned { get; } = [];

        public IReadOnlyList<SourceState> SourceStates(Game game) =>
            [.. _ids.Select(kv => new SourceState(kv.Key, kv.Value is { Length: > 0 } ? kv.Value : null, kv.Value is { Length: > 0 } ? "Lanterna Distante" : null, 2024, Pinned.Exists(p => p.Source == kv.Key), kv.Value?.Length == 0))];

        public Task<IReadOnlyList<MetadataMatch>> SearchSourceAsync(Game game, string source, string term, CancellationToken cancellationToken)
        {
            Searches.Add(source);
            IReadOnlyList<MetadataMatch> results = source switch
            {
                "Steam" => [MetadataMatch.Steam("Lanterna Distante", 4000001) with { Year = 2024, Platforms = "PC" }],
                "IGDB" => [new MetadataMatch("IGDB", "900001", "Lanterna Distante") { Year = 2024, Platforms = "PC" }, new MetadataMatch("IGDB", "900002", "Lanterna Distante 2") { Year = 2026, Platforms = "PC" }],
                _ => [new MetadataMatch(source, "5249", "Lanterna Distante") { Year = 2024 }],
            };
            return Task.FromResult(results);
        }

        public Task<(string Id, GameMetadata Metadata)?> ValidateSourceAsync(string source, string text, CancellationToken cancellationToken) =>
            Task.FromResult<(string Id, GameMetadata Metadata)?>((text.Trim(), new GameMetadata { Source = source, Title = "Lanterna Distante" }));

        public Task<bool> PinSourceAsync(Game game, string source, string? id)
        {
            _ids[source] = id;
            if (id is not null)
            {
                Pinned.Add((source, id));
            }

            return Task.FromResult(true);
        }
    }
}
