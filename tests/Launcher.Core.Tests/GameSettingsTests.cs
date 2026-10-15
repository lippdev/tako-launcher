using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;
using Launcher.Core.Persistence;
using Launcher.Core.Session;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>UX-007: Configurações do Tako + configurações deste jogo → o que a sessão usa.</summary>
public sealed class GameSettingsTests : IDisposable
{
    private static readonly DisplayMode Tv4K = new(3840, 2160, 120);
    private static readonly AudioOutputChoice TvAudio = new("{tv}", "TV (HDMI)");

    private static readonly GlobalSessionSettings Global = new(
        GameScreenEnabled: true,
        GameScreenId: @"\\.\DISPLAY2",
        HideOthers: HideOthersMode.Disconnect,
        GameScreenMode: Tv4K,
        GameScreenHdr: true,
        GameAudio: TvAudio,
        FpsLimit: 60,
        ShowFps: false);

    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public GameSettingsTests() => Directory.CreateDirectory(_root);

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Fact]
    public void No_game_settings_means_the_global_ones()
    {
        var effective = GameSettingsResolver.Resolve(Global, null);

        Assert.Equal(new GameScreenSettings(@"\\.\DISPLAY2", HideOthersMode.Disconnect, Tv4K, true), effective.GameScreen);
        Assert.Null(effective.GameScreenProblem);
        Assert.Equal(TvAudio, effective.GameAudio);
        Assert.Equal(60, effective.FpsLimit);
        Assert.False(effective.FpsLimitFromGame);
        Assert.False(effective.ShowFps);
        Assert.False(effective.ShowFpsFromGame);
        Assert.Null(effective.ExtraArguments);
        Assert.Equal(effective, GameSettingsResolver.Resolve(Global, GameSettingsOverride.None));
    }

    [Fact]
    public void Game_can_skip_the_game_screen()
    {
        var effective = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { UseGameScreen = false });

        Assert.Null(effective.GameScreen);
        Assert.Null(effective.GameScreenProblem);
    }

    [Fact]
    public void Game_can_use_the_game_screen_even_when_it_is_off_globally()
    {
        var effective = GameSettingsResolver.Resolve(Global with { GameScreenEnabled = false }, new GameSettingsOverride { UseGameScreen = true });

        Assert.Equal(@"\\.\DISPLAY2", effective.GameScreen!.DisplayId);
    }

    [Fact]
    public void Game_asking_for_the_game_screen_without_one_chosen_explains_why()
    {
        var noScreen = Global with { GameScreenEnabled = false, GameScreenId = string.Empty };

        var asked = GameSettingsResolver.Resolve(noScreen, new GameSettingsOverride { UseGameScreen = true });
        var notAsked = GameSettingsResolver.Resolve(noScreen with { GameScreenEnabled = true }, null);

        Assert.Null(asked.GameScreen);
        Assert.Equal(GameSettingsResolver.NoScreenChosen, asked.GameScreenProblem);
        Assert.Null(notAsked.GameScreen);
        Assert.Null(notAsked.GameScreenProblem);
    }

    [Theory]
    [InlineData("1920x1080@60", 1920, 1080, 60)]
    [InlineData("", 0, 0, 0)] // manter a atual
    [InlineData("lixo", 3840, 2160, 120)] // valor ilegível: volta ao padrão
    public void Game_resolution_overrides_the_global_mode(string mode, int width, int height, int hz)
    {
        var effective = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { ScreenMode = mode, Hdr = false });

        Assert.Equal(width == 0 ? null : new DisplayMode(width, height, hz), effective.GameScreen!.Mode);
        Assert.False(effective.GameScreen.EnableHdr);
    }

    [Fact]
    public void Game_audio_can_keep_the_current_output_or_pick_another()
    {
        var keep = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { AudioDeviceId = GameSettingsOverride.KeepAudio });
        var headset = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { AudioDeviceId = "{fone}", AudioDeviceName = "Fone" });
        var unnamed = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { AudioDeviceId = "{fone}" });

        Assert.Null(keep.GameAudio);
        Assert.Equal(new AudioOutputChoice("{fone}", "Fone"), headset.GameAudio);
        Assert.Equal("{fone}", unnamed.GameAudio!.Name);
    }

    [Fact]
    public void Game_fps_limit_and_counter_say_where_they_came_from()
    {
        var effective = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { FpsLimit = FpsLimits.MatchDisplay, ShowFps = true });
        var noLimit = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { FpsLimit = 0 });
        var odd = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { FpsLimit = 59 });

        Assert.Equal(FpsLimits.MatchDisplay, effective.FpsLimit);
        Assert.True(effective.FpsLimitFromGame);
        Assert.True(effective.ShowFps);
        Assert.True(effective.ShowFpsFromGame);
        Assert.Equal(0, noLimit.FpsLimit);
        Assert.True(noLimit.FpsLimitFromGame);
        Assert.Equal(60, odd.FpsLimit); // normalizado para uma opção da lista
    }

    [Theory]
    [InlineData(null, null, "")]
    [InlineData("-a", null, "-a")]
    [InlineData(null, " -dx11 ", "-dx11")]
    [InlineData("-a -b", "-windowed", "-a -b -windowed")]
    [InlineData("  ", "   ", "")]
    public void Extra_arguments_go_after_the_game_arguments(string? game, string? extra, string expected)
    {
        var effective = GameSettingsResolver.Resolve(Global, new GameSettingsOverride { ExtraArguments = extra });

        Assert.Equal(expected, effective.Arguments(game));
    }

    [Fact]
    public void Changed_count_ignores_blank_arguments()
    {
        Assert.True(new GameSettingsOverride { ExtraArguments = "  " }.IsDefault);
        Assert.Equal(2, new GameSettingsOverride { Hdr = false, AudioDeviceId = string.Empty }.ChangedCount);
    }

    [Fact]
    public void Confirmed_game_screen_setups_remember_each_mode()
    {
        string saved = GameScreenConfirmations.Remember(null, "tv|current");
        saved = GameScreenConfirmations.Remember(saved, "tv|1920x1080@60");

        Assert.Equal("tv|current", GameScreenConfirmations.Matching(saved, "tv|current"));
        Assert.Equal("TV|1920x1080@60", GameScreenConfirmations.Matching(saved, "TV|1920x1080@60"));
        Assert.Equal(string.Empty, GameScreenConfirmations.Matching(saved, "tv|3840x2160@120"));
        Assert.Equal("tv|current", GameScreenConfirmations.Matching("tv|current", "tv|current")); // valor antigo (uma só)

        for (int i = 0; i < 20; i++)
        {
            saved = GameScreenConfirmations.Remember(saved, $"tv|{i}");
        }

        Assert.Equal(GameScreenConfirmations.MaxRemembered, saved.Split('\n').Length);
        Assert.Equal("tv|19", saved.Split('\n')[0]);
    }

    [Fact]
    public void Store_round_trips_game_settings_and_deletes_defaults()
    {
        var store = new GameSettingsStore(new TakoDatabase(Path.Combine(_root, "tako.db")));
        var settings = new GameSettingsOverride
        {
            UseGameScreen = true,
            ScreenMode = GameSettingsOverride.KeepCurrentMode,
            Hdr = false,
            AudioDeviceId = "{fone}",
            AudioDeviceName = "Fone",
            FpsLimit = FpsLimits.MatchDisplay,
            ShowFps = true,
            ExtraArguments = " -dx11 ",
        };

        Assert.Equal(GameSettingsOverride.None, store.Load("steam:570"));
        store.Save("steam:570", settings);

        Assert.Equal(settings with { ExtraArguments = "-dx11" }, store.Load("steam:570"));
        Assert.Equal(GameSettingsOverride.None, store.Load("steam:730"));

        store.Save("steam:570", new GameSettingsOverride { ShowFps = false });
        Assert.Equal(new GameSettingsOverride { ShowFps = false }, store.Load("steam:570"));

        store.Save("steam:570", GameSettingsOverride.None);
        using var connection = new TakoDatabase(Path.Combine(_root, "tako.db")).Open();
        using var count = connection.CreateCommand();
        count.CommandText = "SELECT COUNT(*) FROM game_settings;";
        Assert.Equal(0L, count.ExecuteScalar());
    }
}
