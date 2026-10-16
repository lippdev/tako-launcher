using Launcher.Core.Emulation;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Sources;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Épico EMU (ADR-0024): tabela de emuladores, detecção por pastas/Scoop/winget/registro, leitura das pastas de ROMs
/// com multi-disco, linha de comando, BIOS, retroarch.cfg e saída do emulador. Tudo com o PC fictício de
/// <c>Fixtures/Emulation/pc.txt</c> (caminhos do Windows, em memória).
/// </summary>
public sealed class EmulationTests
{
    private static readonly EmulationPaths Paths = new(
        @"C:\Program Files",
        @"C:\Program Files (x86)",
        @"C:\Users\Jogador\AppData\Local",
        @"C:\Users\Jogador\AppData\Roaming",
        @"C:\Users\Jogador",
        @"C:\Users\Jogador\Documents",
        @"C:\Users\Jogador\scoop",
        @"C:\ProgramData\scoop");

    private static readonly EmulatorCatalog Catalog = EmulatorCatalog.Default;

    // ------------------------------------------------------------------ Tabela

    [Fact]
    public void Catalog_lists_retroarch_and_the_common_standalone_emulators()
    {
        string[] expected = ["retroarch", "dolphin", "pcsx2", "duckstation", "ppsspp", "rpcs3", "cemu", "ryujinx", "yuzu-forks", "xemu", "xenia", "melonds", "mgba", "flycast", "redream", "mame", "vita3k"];
        Assert.Equal(expected, Catalog.Emulators.Select(e => e.Id));
        Assert.Equal(1, Catalog.Version);
    }

    [Fact]
    public void Catalog_entries_are_consistent()
    {
        var systems = ScreenScraperSystems.All.Select(s => s.Key).ToHashSet();
        foreach (var emulator in Catalog.Emulators)
        {
            Assert.NotEmpty(emulator.Executables);
            Assert.All(emulator.Executables, exe => Assert.EndsWith(".exe", exe, StringComparison.OrdinalIgnoreCase));
            Assert.True(emulator.Arguments.Contains(LaunchArguments.Rom, StringComparison.Ordinal) || emulator.Arguments.Contains(LaunchArguments.RomName, StringComparison.Ordinal), emulator.Id);
            Assert.Empty(LaunchArguments.Unknown(emulator.Arguments + " " + emulator.ConfigArgument));
            Assert.NotEmpty(emulator.Folders);
            Assert.All(emulator.Systems, s => Assert.Contains(s, systems));
            Assert.Equal(emulator.UsesCores, emulator.Systems.Count == 0);
            Assert.All(emulator.Folders, f => Assert.NotNull(Paths.Expand(f)));
        }

        Assert.All(Catalog.RetroArchCores, c => Assert.All(c.Systems, s => Assert.Contains(s, systems)));
        Assert.All(Catalog.RetroArchDatabases.Values, s => Assert.Contains(s, systems));
        Assert.All(Catalog.SystemFolders.Keys, s => Assert.Contains(s, systems));
        Assert.All(Catalog.Preferred, p =>
        {
            Assert.Contains(p.Key, systems);
            Assert.All(p.Value, id => Assert.NotNull(Catalog.Find(id)));
        });
        Assert.All(Catalog.Bios, b =>
        {
            Assert.NotNull(Catalog.Find(b.Emulator));
            Assert.Contains(b.System, Catalog.Find(b.Emulator)!.Systems);
        });

        // Todo sistema do Tako tem pasta reconhecida e alguém que o abre (emulador próprio ou core sugerido), menos o 3DS.
        Assert.All(systems, s => Assert.NotNull(Catalog.SystemForFolder(s)));
        Assert.All(systems.Where(s => s != "3ds"), s => Assert.True(
            Catalog.Emulators.Any(e => e.Systems.Contains(s)) || Catalog.RetroArchCores.Any(c => c.Systems.Contains(s)),
            $"ninguém abre {s}"));
    }

    [Theory]
    [InlineData("psx", "ps1")]
    [InlineData("PSX", "ps1")]
    [InlineData("genesis", "megadrive")]
    [InlineData("gc", "gamecube")]
    [InlineData("sfc", "snes")]
    [InlineData("mame", "arcade")]
    [InlineData("fotos", null)]
    public void Folder_names_map_to_systems(string folder, string? system) => Assert.Equal(system, Catalog.SystemForFolder(folder));

    // ------------------------------------------------------------------ Detecção

    [Fact]
    public void Detects_emulators_by_folder_scoop_winget_and_registry()
    {
        var inventory = Detector().Detect(EmulationConfig.Default);

        Assert.Equal(["retroarch", "dolphin", "pcsx2", "duckstation"], inventory.Emulators.Select(e => e.Id));
        var retroArch = inventory.Find("retroarch")!;
        Assert.Equal(EmulatorOrigin.Folder, retroArch.Origin);
        Assert.Equal(@"C:\Program Files\RetroArch\retroarch.exe", retroArch.ExePath);
        Assert.Equal(EmulatorOrigin.Scoop, inventory.Find("pcsx2")!.Origin);
        Assert.Equal(@"C:\Users\Jogador\scoop\apps\pcsx2\current\pcsx2-qt.exe", inventory.Find("pcsx2")!.ExePath);
        Assert.Equal(EmulatorOrigin.Winget, inventory.Find("duckstation")!.Origin);
        Assert.Equal(EmulatorOrigin.Registry, inventory.Find("dolphin")!.Origin);
        Assert.Equal(@"D:\Emus\Dolphin-x64\Dolphin.exe", inventory.Find("dolphin")!.ExePath);
    }

    [Fact]
    public void RetroArch_cores_come_from_the_cores_folder_and_info_files()
    {
        var retroArch = Detector().Detect(EmulationConfig.Default).Find("retroarch")!;

        // O core sem sistema conhecido fica de fora; o fbneo sem .info usa a tabela de sugestões.
        Assert.Equal(["fbneo", "snes9x", "swanstation"], retroArch.Cores.Select(c => c.Id));
        var snes9x = retroArch.Cores.Single(c => c.Id == "snes9x");
        Assert.Equal("Snes9x", snes9x.Name);
        Assert.Equal(["snes"], snes9x.Systems);
        Assert.Contains(".sfc", snes9x.Extensions);
        Assert.Equal(@"C:\Program Files\RetroArch\cores\snes9x_libretro.dll", snes9x.CorePath);
        var swan = retroArch.Cores.Single(c => c.Id == "swanstation");
        Assert.Equal(2, swan.Firmware.Count);
        Assert.False(swan.Firmware[0].Optional);
        Assert.True(swan.Firmware[1].Optional);
        Assert.Equal(["arcade", "neogeo"], retroArch.Cores.Single(c => c.Id == "fbneo").Systems);
        Assert.Equal(["arcade", "neogeo", "snes", "ps1"], retroArch.Systems);
    }

    [Fact]
    public void Chosen_executable_and_custom_emulators_win()
    {
        var disk = FakeDisk.Load();
        disk.Add(@"G:\Outro\RetroArch\retroarch.exe");
        var custom = CustomEmulator.Create(@"D:\Emus\MeuEmulador\meu.exe", ["dreamcast"]);
        var config = EmulationConfig.Default.WithEmulatorPath("retroarch", @"G:\Outro\RetroArch\retroarch.exe") with { CustomEmulators = [custom] };

        var inventory = new EmulatorDetector(disk, Registry(), Paths, Catalog).Detect(config);

        var retroArch = inventory.Find("retroarch")!;
        Assert.Equal(EmulatorOrigin.Chosen, retroArch.Origin);
        Assert.Empty(retroArch.Cores); // a outra pasta não tem cores
        var mine = inventory.Find(custom.Id)!;
        Assert.Equal(EmulatorOrigin.Custom, mine.Origin);
        Assert.Equal("meu", mine.Name);
        Assert.True(mine.Serves("dreamcast"));

        // Executável escolhido que sumiu: volta para a detecção normal.
        var gone = EmulationConfig.Default.WithEmulatorPath("retroarch", @"Z:\sumiu\retroarch.exe");
        Assert.Equal(EmulatorOrigin.Folder, new EmulatorDetector(disk, Registry(), Paths, Catalog).Detect(gone).Find("retroarch")!.Origin);
    }

    [Fact]
    public void Nothing_installed_means_empty_inventory()
    {
        var inventory = new EmulatorDetector(new FakeDisk(), new FakeRegistry(), Paths, Catalog).Detect(EmulationConfig.Default);
        Assert.Empty(inventory.Emulators);
        Assert.Empty(inventory.OptionsFor("snes"));
    }

    [Fact]
    public void Options_follow_the_preference_table_and_suggested_cores()
    {
        var inventory = Detector().Detect(EmulationConfig.Default);

        Assert.Equal(["DuckStation", "RetroArch · SwanStation"], inventory.OptionsFor("ps1").Select(o => o.Label));
        Assert.Equal(["PCSX2"], inventory.OptionsFor("ps2").Select(o => o.Label));
        Assert.Equal(["RetroArch · Snes9x"], inventory.OptionsFor("snes").Select(o => o.Label));
        Assert.Equal(["Dolphin"], inventory.OptionsFor("gamecube").Select(o => o.Label));
        Assert.Empty(inventory.OptionsFor("dreamcast"));
        Assert.Contains("ps1", inventory.CoveredSystems);
        Assert.DoesNotContain("dreamcast", inventory.CoveredSystems);
    }

    // ------------------------------------------------------------------ Pastas de ROMs

    [Fact]
    public void Root_folder_with_system_subfolders_lists_one_game_per_title()
    {
        var entries = new RomScanner(FakeDisk.Load(), Catalog).Scan([new RomFolder(@"E:\ROMs")]);

        Assert.Equal(
            [
                ("arcade", "lutaneon"), ("dreamcast", "Mar Profundo"), ("gamecube", "Ilha Flutuante"), ("ps1", "Saga Estelar"),
                ("ps1", "Torre Antiga"), ("ps1", "Vale Sombrio"), ("ps2", "Rua Veloz"), ("snes", "Corrida Pixel"), ("snes", "Lanterna Distante"),
            ],
            entries.Select(e => (e.System, e.Title)).OrderBy(e => e.System, StringComparer.Ordinal).ThenBy(e => e.Title, StringComparer.Ordinal));
        Assert.DoesNotContain(entries, e => e.Path.Contains("media", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Multi_disc_games_are_grouped()
    {
        var entries = new RomScanner(FakeDisk.Load(), Catalog).Scan([new RomFolder(@"E:\ROMs\psx")]);

        // .cue esconde as trilhas .bin; o .m3u esconde os discos que lista; "(Disc N)" soltos viram um jogo só.
        var vale = entries.Single(e => e.Title == "Vale Sombrio");
        Assert.Equal(@"E:\ROMs\psx\Vale Sombrio (USA).cue", vale.Path);
        var saga = entries.Single(e => e.Title == "Saga Estelar");
        Assert.EndsWith(".m3u", saga.Path, StringComparison.Ordinal);
        var torre = entries.Single(e => e.Title == "Torre Antiga");
        Assert.Equal(@"E:\ROMs\psx\Torre Antiga (Europe) (Disc 1).chd", torre.Path);
        Assert.Equal(2, torre.Discs.Count);
        Assert.EndsWith("(Disc 2).chd", torre.Discs[1], StringComparison.Ordinal);
        Assert.Equal(3, entries.Count);
        Assert.Equal("2 discos", EmulationLibrarySource.ToGame(torre).SizeLabel);
    }

    [Fact]
    public void Playlists_are_parsed()
    {
        var scanner = new RomScanner(FakeDisk.Load(), Catalog);
        Assert.Equal(
            [@"E:\ROMs\psx\Vale Sombrio (USA) (Track 1).bin", @"E:\ROMs\psx\Vale Sombrio (USA) (Track 2).bin"],
            scanner.ReferencedFiles(@"E:\ROMs\psx\Vale Sombrio (USA).cue"));
        Assert.Equal(2, scanner.ReferencedFiles(@"E:\ROMs\psx\Saga Estelar\Saga Estelar (USA).m3u").Count);
        Assert.Equal(
            [@"E:\ROMs\dreamcast\Mar Profundo (USA)\track01.bin", @"E:\ROMs\dreamcast\Mar Profundo (USA)\track02.raw", @"E:\ROMs\dreamcast\Mar Profundo (USA)\track03.bin"],
            scanner.ReferencedFiles(@"E:\ROMs\dreamcast\Mar Profundo (USA)\Mar Profundo (USA).gdi"));
        Assert.Equal(2, RomScanner.DiscNumber("Jogo (USA) (Disc 2 of 3).chd"));
        Assert.Equal(RomScanner.DiscBase("Jogo (USA) (Disc 1).chd"), RomScanner.DiscBase("jogo (USA) (CD 2).chd"));
    }

    [Fact]
    public void Single_system_folder_and_archive_rules()
    {
        var disk = FakeDisk.Load();
        var scanner = new RomScanner(disk, Catalog);

        var gba = scanner.Scan([new RomFolder(@"F:\Portateis", "gba")]);
        Assert.Equal(["Aventura de Bolso"], gba.Select(e => e.Title)); // o .sav fica de fora

        // .zip vale em sistemas de cartucho e arcade, não em sistemas de disco.
        Assert.Contains(".zip", RomScanner.Extensions(ScreenScraperSystems.ForKey("snes")!));
        Assert.Contains(".zip", RomScanner.Extensions(ScreenScraperSystems.ForKey("arcade")!));
        Assert.DoesNotContain(".zip", RomScanner.Extensions(ScreenScraperSystems.ForKey("ps2")!));

        // Pasta desconhecida, nem sistema nem raiz com subpastas reconhecidas: nada.
        Assert.Empty(scanner.Scan([new RomFolder(@"E:\ROMs\desconhecido")]));
        Assert.Empty(scanner.Scan([new RomFolder(@"Z:\nao-existe")]));
    }

    [Fact]
    public void Rescan_reuses_unchanged_folders()
    {
        var disk = FakeDisk.Load();
        var cache = new RomScanCache();
        var scanner = new RomScanner(disk, Catalog, cache);

        int first = scanner.Scan([new RomFolder(@"E:\ROMs")]).Count;
        Assert.Equal(0, cache.Hits);
        Assert.True(cache.Misses > 0);

        Assert.Equal(first, scanner.Scan([new RomFolder(@"E:\ROMs")]).Count);
        Assert.Equal(0, cache.Misses); // nada mudou: nenhuma pasta listada de novo

        disk.Add(@"E:\ROMs\snes\Nova Aventura (USA).sfc"); // muda a data da pasta snes
        var again = scanner.Scan([new RomFolder(@"E:\ROMs")]);
        Assert.Equal(first + 1, again.Count);
        Assert.Equal(1, cache.Misses);
        Assert.Contains(again, e => e.Title == "Nova Aventura");
    }

    [Fact]
    public void Emulation_source_turns_roms_into_games_with_stable_ids()
    {
        var disk = FakeDisk.Load();
        var inventory = new EmulatorDetector(disk, Registry(), Paths, Catalog).Detect(EmulationConfig.Default);
        var config = EmulationConfig.Default with { RomFolders = [new RomFolder(@"E:\ROMs")] };
        var source = new EmulationLibrarySource(() => config, new RomScanner(disk, Catalog), () => inventory);

        var scan = source.Scan();

        Assert.Equal(GameStore.Emulation, source.Store);
        Assert.Equal(9, scan.Games.Count);
        Assert.True(scan.ClientInstalled);
        Assert.Equal("Sem emulador para Dreamcast", scan.Notice);
        var lanterna = scan.Games.Single(g => g.Title == "Lanterna Distante");
        Assert.Equal("snes", lanterna.Platform);
        Assert.True(lanterna.IsRom);
        Assert.True(lanterna.CanPlay);
        Assert.Equal(lanterna.Id, EmulationLibrarySource.GameId("snes", @"e:/roms/SNES/Lanterna Distante (USA).sfc"));
        Assert.StartsWith("emu:snes:", lanterna.Id, StringComparison.Ordinal);
        Assert.Equal("Emulação", Presentation.GameText.StoreName(GameStore.Emulation));

        var empty = new EmulationLibrarySource(() => EmulationConfig.Default, new RomScanner(disk, Catalog), () => EmulatorInventory.Empty).Scan();
        Assert.Empty(empty.Games);
        Assert.False(empty.ClientInstalled);
        Assert.Contains("Configurações › Emulação", empty.Notice, StringComparison.Ordinal);
        Assert.StartsWith("Nenhuma pasta de ROMs", new LibrarySourceReport(GameStore.Emulation, true, 0, false, empty.Notice, false).Summary, StringComparison.Ordinal);
    }

    [Fact]
    public void Library_filter_by_system()
    {
        var games = new RomScanner(FakeDisk.Load(), Catalog).Scan([new RomFolder(@"E:\ROMs")]).Select(EmulationLibrarySource.ToGame).ToList();
        var facets = LibraryQuery.Systems(games, LibraryFacts.Of);

        // Na ordem da tabela de sistemas: SNES antes de GameCube, PS1 e PS2 juntos, arcade no fim.
        Assert.Equal([("snes", 2), ("gamecube", 1), ("dreamcast", 1), ("ps1", 3), ("ps2", 1), ("arcade", 1)], facets.Select(f => (f.Name, f.Count)));
        var criteria = LibraryCriteria.None.ToggleSystem("ps1");
        Assert.Equal(1, criteria.ActiveFilterCount);
        Assert.Equal(3, games.Count(g => criteria.Matches(LibraryFacts.Of(g))));
        Assert.Equal(6, games.Count(g => criteria.ToggleSystem("snes").ToggleSystem("ps2").Matches(LibraryFacts.Of(g))));
        Assert.Empty(criteria.ClearFilters().Systems);
        Assert.False(criteria.Matches(LibraryFacts.Of(new Game { Id = "pc", Title = "Jogo de PC", Store = GameStore.Steam, Accent = "#000000" })));
    }

    // ------------------------------------------------------------------ Linha de comando

    [Fact]
    public void Arguments_expand_and_quote_paths()
    {
        var values = LaunchArguments.For(@"E:\ROMs\snes\Lanterna Distante (USA).sfc", @"C:\RA\cores\snes9x_libretro.dll", null, "-f");

        Assert.Equal(@"-L ""C:\RA\cores\snes9x_libretro.dll"" ""E:\ROMs\snes\Lanterna Distante (USA).sfc"" -f", LaunchArguments.Expand("-L {core} {rom} {fullscreen}", values));
        // Aspas no modelo não duplicam; {config} sem valor some.
        Assert.Equal(@"""E:\ROMs\snes\Lanterna Distante (USA).sfc"" -f", LaunchArguments.Expand(@"""{rom}"" {config} {fullscreen}", values));
        Assert.Equal(@"-rompath ""E:\ROMs\snes"" ""Lanterna Distante (USA)""", LaunchArguments.Expand("-rompath {romdir} {romname}", values));
        Assert.Equal(@"""E:\ROMs\snes\Lanterna Distante (USA).sfc""", LaunchArguments.Expand("{ROM}", values));
        Assert.Equal(["{ROMPATH}", "{x}"], LaunchArguments.Unknown("{ROMPATH} {rom} {x} {fullscreen}"));
        Assert.Empty(LaunchArguments.Unknown("{rom} {romdir} {romname} {core} {config} {fullscreen}"));
    }

    [Fact]
    public void Resolver_picks_the_suggested_emulator_and_builds_the_command()
    {
        var (disk, inventory) = Setup();

        var ps1 = Assert.IsType<EmulatorLaunch>(Resolve(Rom("ps1", @"E:\ROMs\psx\Vale Sombrio (USA).cue"), EmulationConfig.Default, disk, inventory));
        Assert.Equal("duckstation", ps1.Emulator.Id);
        Assert.Equal(@"-batch -fullscreen -- ""E:\ROMs\psx\Vale Sombrio (USA).cue""", ps1.Arguments);
        Assert.Equal(WinPath.Directory(ps1.Executable), ps1.WorkingDirectory);
        Assert.Single(ps1.MissingBios); // DuckStation sem BIOS: avisa onde colocar
        Assert.Contains(@"C:\Users\Jogador\AppData\Local\Microsoft\WinGet\Packages\Stenzek.DuckStation_Microsoft.Winget.Source_8wekyb3d8bbwe\bios", ps1.MissingBios[0], StringComparison.Ordinal);

        var snes = Assert.IsType<EmulatorLaunch>(Resolve(Rom("snes", @"E:\ROMs\snes\Lanterna Distante (USA).sfc"), EmulationConfig.Default, disk, inventory));
        Assert.Equal(@"C:\Program Files\RetroArch\retroarch.exe", snes.Executable);
        Assert.Equal(@"-L ""C:\Program Files\RetroArch\cores\snes9x_libretro.dll"" ""E:\ROMs\snes\Lanterna Distante (USA).sfc"" -f", snes.Arguments);
        Assert.Empty(snes.MissingBios);

        var ps2 = Assert.IsType<EmulatorLaunch>(Resolve(Rom("ps2", @"E:\ROMs\ps2\Rua Veloz (USA).iso"), EmulationConfig.Default with { Fullscreen = false }, disk, inventory));
        Assert.Equal(@"-batch -nogui -- ""E:\ROMs\ps2\Rua Veloz (USA).iso""", ps2.Arguments);
        Assert.Empty(ps2.MissingBios); // BIOS em Documentos\PCSX2\bios
    }

    [Fact]
    public void System_and_game_choices_override_the_default()
    {
        var (disk, inventory) = Setup();
        var game = Rom("ps1", @"E:\ROMs\psx\Vale Sombrio (USA).cue");

        // Sistema: RetroArch com SwanStation; falta a BIOS obrigatória do core (a opcional não conta).
        var system = EmulationConfig.Default.WithSystemChoice("ps1", new EmulatorChoice("retroarch", "swanstation"));
        var retro = Assert.IsType<EmulatorLaunch>(Resolve(game, system, disk, inventory));
        Assert.Equal("RetroArch · SwanStation", retro.Option.Label);
        Assert.Equal([@"BIOS do PS1 (EUA): coloque scph5501.bin em C:\Program Files\RetroArch\system"], retro.MissingBios);

        // Jogo: volta para o DuckStation com argumentos próprios; o .cfg extra só vale no RetroArch.
        var perGame = system.WithGame(game.Id, new GameEmulation(new EmulatorChoice("duckstation"), "{rom} -turbo"));
        Assert.Equal(@"""E:\ROMs\psx\Vale Sombrio (USA).cue"" -turbo", Assert.IsType<EmulatorLaunch>(Resolve(game, perGame, disk, inventory)).Arguments);

        var cfg = system.WithGame(game.Id, new GameEmulation(ConfigFile: @"E:\cfg\vale.cfg"));
        Assert.EndsWith(@"-f --appendconfig ""E:\cfg\vale.cfg""", Assert.IsType<EmulatorLaunch>(Resolve(game, cfg, disk, inventory)).Arguments, StringComparison.Ordinal);

        // Argumentos do emulador e extras do sistema.
        var custom = EmulationConfig.Default.WithEmulatorArguments("duckstation", "-batch {rom}") with { SystemArguments = new Dictionary<string, string> { ["ps1"] = "-fastboot" } };
        Assert.Equal(@"-batch ""E:\ROMs\psx\Vale Sombrio (USA).cue"" -fastboot", Assert.IsType<EmulatorLaunch>(Resolve(game, custom, disk, inventory)).Arguments);

        // Escolha que não existe mais (core removido): fica a sugestão.
        var gone = EmulationConfig.Default.WithSystemChoice("ps1", new EmulatorChoice("retroarch", "beetle_que_sumiu"));
        Assert.Equal("RetroArch · SwanStation", Assert.IsType<EmulatorLaunch>(Resolve(game, gone, disk, inventory)).Option.Label);
        Assert.Equal("duckstation", Assert.IsType<EmulatorLaunch>(Resolve(game, EmulationConfig.Default.WithSystemChoice("ps1", new EmulatorChoice("xenia")), disk, inventory)).Emulator.Id);

        // Limpar as escolhas do jogo tira a entrada.
        Assert.Empty(perGame.WithGame(game.Id, new GameEmulation()).Games);
    }

    [Fact]
    public void Resolver_explains_what_is_missing()
    {
        var (disk, inventory) = Setup();

        var dreamcast = Assert.IsType<EmulatorUnavailable>(Resolve(Rom("dreamcast", @"E:\ROMs\dreamcast\Mar Profundo (USA)\Mar Profundo (USA).gdi"), EmulationConfig.Default, disk, inventory));
        Assert.Equal("Nenhum emulador para Dreamcast", dreamcast.Title);
        Assert.Contains("Flycast, Redream ou RetroArch com o core Flycast", dreamcast.Message, StringComparison.Ordinal);
        Assert.Contains("Configurações › Emulação", dreamcast.Message, StringComparison.Ordinal);

        var gone = Assert.IsType<EmulatorUnavailable>(Resolve(Rom("snes", @"E:\ROMs\snes\Sumiu.sfc"), EmulationConfig.Default, disk, inventory));
        Assert.Equal("Arquivo não encontrado", gone.Title);

        Assert.IsType<EmulatorUnavailable>(Resolve(Rom("console-que-nao-existe", @"E:\ROMs\snes\Lanterna Distante (USA).sfc"), EmulationConfig.Default, disk, inventory));
    }

    // ------------------------------------------------------------------ RetroArch.cfg e saída

    [Fact]
    public void RetroArch_hotkeys_never_overwrite_user_choices()
    {
        string original = FakeDisk.Fixture("retroarch.cfg");
        var change = RetroArch.Merge(original, Catalog.RetroArchHotkeys);
        Assert.Equal(["input_quit_gamepad_combo", "quit_press_twice"], change.Applied);
        Assert.Contains("input_quit_gamepad_combo = \"4\"", change.Text, StringComparison.Ordinal);
        Assert.Contains("quit_press_twice = \"false\"", change.Text, StringComparison.Ordinal);
        Assert.Contains("video_fullscreen = \"false\"", change.Text, StringComparison.Ordinal);

        // Valor escolhido pelo usuário fica; o que falta é acrescentado; aplicar de novo não muda nada.
        var user = RetroArch.Merge("input_quit_gamepad_combo = \"2\"\r\n", Catalog.RetroArchHotkeys);
        Assert.Equal(["quit_press_twice"], user.Applied);
        Assert.Equal(["input_quit_gamepad_combo"], user.KeptUserValue);
        Assert.Equal("input_quit_gamepad_combo = \"2\"\r\nquit_press_twice = \"false\"\r\n", user.Text);
        var again = RetroArch.Merge(change.Text, Catalog.RetroArchHotkeys);
        Assert.False(again.Changed);
        Assert.Equal(change.Text, again.Text);
    }

    [Fact]
    public void RetroArch_folders_follow_the_config()
    {
        var disk = FakeDisk.Load();
        Assert.Equal(@"C:\Program Files\RetroArch\retroarch.cfg", RetroArch.ConfigFile(disk, @"C:\Program Files\RetroArch", Paths));
        Assert.Null(RetroArch.ConfigFile(disk, @"D:\SemConfig", Paths));
        Assert.Equal(@"C:\RA\system", RetroArch.SystemFolder(@"C:\RA", null));
        Assert.Equal(@"C:\RA\bios", RetroArch.SystemFolder(@"C:\RA", new Dictionary<string, string> { ["system_directory"] = @":\bios" }));
        Assert.Equal(@"E:\BIOS", RetroArch.SystemFolder(@"C:\RA", new Dictionary<string, string> { ["system_directory"] = @"E:\BIOS" }));
    }

    [Fact]
    public void Exit_sequence_closes_gracefully_before_asking()
    {
        var retroArch = new EmulatorExitSequence(Catalog.Find("retroarch")!);
        Assert.Equal(EmulatorExitStep.CloseWindow, retroArch.Current);
        Assert.Equal(EmulatorExitStep.SendExitKey, retroArch.Next(stillRunning: true));
        Assert.Equal(EmulatorExitStep.AskToForceClose, retroArch.Next(stillRunning: true));
        Assert.Equal(EmulatorExitStep.Done, retroArch.Next(stillRunning: false));

        var noKey = new EmulatorExitSequence(Catalog.Find("ppsspp")!);
        Assert.Equal(EmulatorExitStep.AskToForceClose, noKey.Next(stillRunning: true));
        Assert.Equal(EmulatorExitStep.Done, new EmulatorExitSequence(Catalog.Find("ppsspp")!).Next(stillRunning: false));
        Assert.Equal(GamepadButtons.View | GamepadButtons.Start, EmulatorExitSequence.Chord);
        Assert.Equal(2, Catalog.Find("retroarch")!.ExitKeyPresses);
    }

    [Fact]
    public void Config_round_trips_through_the_file()
    {
        string file = Path.Combine(Path.GetTempPath(), $"tako-emulation-{Guid.NewGuid():N}.json");
        try
        {
            var store = new EmulationConfigStore(file);
            Assert.Same(EmulationConfig.Default, store.Load());
            var config = EmulationConfig.Default.WithSystemChoice("ps1", new EmulatorChoice("retroarch", "swanstation"))
                .WithGame("emu:ps1:abc", new GameEmulation(Arguments: "{rom}")) with
            { RomFolders = [new RomFolder(@"E:\ROMs"), new RomFolder(@"F:\GBA", "gba")], ExitChord = false };
            store.Save(config);

            var loaded = store.Load();
            Assert.Equal(config.RomFolders, loaded.RomFolders);
            Assert.Equal("swanstation", loaded.SystemChoices["ps1"].Core);
            Assert.Equal("{rom}", loaded.Games["emu:ps1:abc"].Arguments);
            Assert.False(loaded.ExitChord);
            Assert.True(loaded.Fullscreen);

            File.WriteAllText(file, "{ quebrado");
            Assert.Same(EmulationConfig.Default, store.Load());
            Assert.True(File.Exists(file + ".invalido"));
        }
        finally
        {
            File.Delete(file);
            File.Delete(file + ".invalido");
        }
    }

    // ------------------------------------------------------------------ Apoio

    private static EmulatorDetector Detector() => new(FakeDisk.Load(), Registry(), Paths, Catalog);

    private static (FakeDisk Disk, EmulatorInventory Inventory) Setup()
    {
        var disk = FakeDisk.Load();
        return (disk, new EmulatorDetector(disk, Registry(), Paths, Catalog).Detect(EmulationConfig.Default));
    }

    private static EmulatorResolution Resolve(Game game, EmulationConfig config, FakeDisk disk, EmulatorInventory inventory) =>
        EmulatorResolver.Resolve(game, config, inventory, disk, Paths);

    private static Game Rom(string system, string path) => EmulationLibrarySource.ToGame(new RomEntry(path, system, RomTitle.FromFileName(WinPath.FileName(path)), [path]));

    /// <summary>Dolphin em "Aplicativos instalados"; uma entrada sem pasta e outra de outro programa são ignoradas.</summary>
    private static FakeRegistry Registry() => new FakeRegistry()
        .Set(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Dolphin", "DisplayName", "Dolphin 2412")
        .Set(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Dolphin", "DisplayIcon", @"""D:\Emus\Dolphin-x64\Dolphin.exe"",0")
        .Set(@"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Cemu", "DisplayName", "Cemu")
        .Set(@"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Editor", "DisplayName", "Editor de fotos")
        .Set(@"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Editor", "InstallLocation", @"D:\Emus\Dolphin-x64");

    private sealed class FakeRegistry : IRegistryReader
    {
        private readonly Dictionary<string, Dictionary<string, string>> _keys = new(StringComparer.OrdinalIgnoreCase);

        public FakeRegistry Set(string key, string name, string value)
        {
            if (!_keys.TryGetValue(key, out var values))
            {
                _keys[key] = values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            }

            values[name] = value;
            return this;
        }

        public IReadOnlyList<string> SubKeyNames(string keyPath) =>
            [.. _keys.Keys
                .Where(k => k.StartsWith(keyPath + "\\", StringComparison.OrdinalIgnoreCase))
                .Select(k => k[(keyPath.Length + 1)..].Split('\\')[0])
                .Distinct(StringComparer.OrdinalIgnoreCase)];

        public string? GetString(string keyPath, string valueName) =>
            _keys.TryGetValue(keyPath, out var values) ? values.GetValueOrDefault(valueName) : null;
    }

    /// <summary>Disco em memória com caminhos do Windows; cada arquivo novo muda a data da pasta (como no NTFS).</summary>
    private sealed class FakeDisk : IEmulationFileSystem
    {
        private static readonly string Folder = Path.Combine(AppContext.BaseDirectory, "Fixtures", "Emulation");
        private readonly Dictionary<string, (string Path, string? Content)> _files = new(StringComparer.Ordinal);
        private readonly Dictionary<string, DateTimeOffset> _stamps = new(StringComparer.Ordinal);
        private DateTimeOffset _clock = new(2026, 10, 1, 12, 0, 0, TimeSpan.Zero);

        public static string Fixture(string name) => File.ReadAllText(Path.Combine(Folder, name));

        public static FakeDisk Load()
        {
            var disk = new FakeDisk();
            foreach (string raw in File.ReadAllLines(Path.Combine(Folder, "pc.txt")))
            {
                string line = raw.Trim();
                if (line.Length == 0 || line.StartsWith('#'))
                {
                    continue;
                }

                string[] parts = line.Split(" => ");
                disk.Add(parts[0].Trim(), parts.Length > 1 ? Fixture(parts[1].Trim()) : null);
            }

            return disk;
        }

        public void Add(string path, string? content = null)
        {
            _clock = _clock.AddMinutes(1);
            _files[WinPath.Normalize(path)] = (path, content);
            for (string folder = WinPath.Directory(path); folder.Length > 0; folder = WinPath.Directory(folder))
            {
                _stamps.TryAdd(WinPath.Normalize(folder), _clock);
            }

            _stamps[WinPath.Normalize(WinPath.Directory(path))] = _clock;
        }

        public bool FileExists(string path) => _files.ContainsKey(WinPath.Normalize(path));

        public bool DirectoryExists(string path) => _stamps.ContainsKey(WinPath.Normalize(path));

        public IReadOnlyList<string> Files(string directory)
        {
            string folder = WinPath.Normalize(directory);
            return [.. _files.Values.Where(f => WinPath.Normalize(WinPath.Directory(f.Path)) == folder).Select(f => WinPath.Combine(directory, WinPath.FileName(f.Path)))];
        }

        public IReadOnlyList<string> Directories(string directory)
        {
            string folder = WinPath.Normalize(directory);
            return [.. _stamps.Keys.Where(d => WinPath.Normalize(WinPath.Directory(d)) == folder && d != folder)
                .Select(d => WinPath.Combine(directory, OriginalName(d)))];
        }

        public string? ReadText(string path) => _files.TryGetValue(WinPath.Normalize(path), out var file) ? file.Content : null;

        public DateTimeOffset? LastWrite(string path) =>
            _stamps.TryGetValue(WinPath.Normalize(path), out var stamp) ? stamp : FileExists(path) ? _clock : null;

        // Nome da pasta com as maiúsculas originais (as chaves estão normalizadas).
        private string OriginalName(string normalizedFolder)
        {
            foreach (var (path, _) in _files.Values)
            {
                for (string folder = WinPath.Directory(path); folder.Length > 0; folder = WinPath.Directory(folder))
                {
                    if (WinPath.Normalize(folder) == normalizedFolder)
                    {
                        return WinPath.FileName(folder);
                    }
                }
            }

            return WinPath.FileName(normalizedFolder);
        }
    }
}
