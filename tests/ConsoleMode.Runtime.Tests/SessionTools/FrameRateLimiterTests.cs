using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class FpsLimitsTests
{
    [Fact]
    public void Options_follow_the_control_center_order()
    {
        Assert.Equal([0, 30, 40, 45, 60, 90, 120, 144, FpsLimits.MatchDisplay], FpsLimits.Options);
        Assert.Equal("Sem limite", FpsLimits.Label(0));
        Assert.Equal("60 FPS", FpsLimits.Label(60));
        Assert.Equal("Tela − 3 (VRR)", FpsLimits.Label(FpsLimits.MatchDisplay));
        Assert.Equal("Tela − 3 (117 FPS)", FpsLimits.Label(FpsLimits.MatchDisplay, 120));
    }

    [Theory]
    [InlineData(-5, 0)]
    [InlineData(-1, -1)]
    [InlineData(0, 0)]
    [InlineData(60, 60)]
    [InlineData(59, 60)]
    [InlineData(75, 60)]
    [InlineData(100, 90)]
    [InlineData(165, 144)]
    [InlineData(500, 144)]
    public void Saved_values_snap_to_an_offered_limit(int saved, int expected) =>
        Assert.Equal(expected, FpsLimits.Normalize(saved));

    [Fact]
    public void Left_and_right_stop_at_the_ends()
    {
        Assert.Equal(0, FpsLimits.Step(0, -1));
        Assert.Equal(30, FpsLimits.Step(0, 1));
        Assert.Equal(45, FpsLimits.Step(60, -1));
        Assert.Equal(FpsLimits.MatchDisplay, FpsLimits.Step(144, 1));
        Assert.Equal(FpsLimits.MatchDisplay, FpsLimits.Step(FpsLimits.MatchDisplay, 1));
    }

    [Fact]
    public void A_cycles_back_to_no_limit_after_the_last()
    {
        Assert.Equal(30, FpsLimits.Cycle(0));
        Assert.Equal(0, FpsLimits.Cycle(FpsLimits.MatchDisplay));
    }

    [Theory]
    [InlineData(0, 144, 0)]
    [InlineData(60, null, 60)]
    [InlineData(-1, 144, 141)]
    [InlineData(-1, 60, 57)]
    [InlineData(-1, null, null)]
    [InlineData(-1, 20, null)]
    public void Match_display_resolves_to_refresh_minus_three(int option, int? refresh, int? expected) =>
        Assert.Equal(expected, FpsLimits.Resolve(option, refresh));
}

public class GameExecutablesTests
{
    [Theory]
    [InlineData(@"C:\Games\Hades\Hades.exe", @"C:\Games\Hades\Hades.exe")]
    [InlineData(@"""C:\Games\Hades\Hades.exe""", @"C:\Games\Hades\Hades.exe")]
    [InlineData("steam://rungameid/570", null)]
    [InlineData(@"C:\Users\a\Desktop\Jogo.lnk", null)]
    [InlineData(null, null)]
    public void Only_an_exe_launch_target_is_known_before_the_game_starts(string? target, string? expected) =>
        Assert.Equal(expected, GameExecutables.FromLaunchTarget(target));

    [Fact]
    public void Pick_skips_anticheat_crash_handlers_and_launchers()
    {
        string?[] paths =
        [
            @"C:\Games\Elden\EasyAntiCheat_EOS.exe",
            @"C:\Games\Elden\UnityCrashHandler64.exe",
            null,
            @"C:\Games\Elden\eldenring.exe",
        ];
        Assert.Equal(@"C:\Games\Elden\eldenring.exe", GameExecutables.Pick(paths));
    }

    [Fact]
    public void Pick_falls_back_to_a_helper_and_returns_null_for_nothing()
    {
        Assert.Equal(@"C:\G\Launcher.exe", GameExecutables.Pick([@"C:\G\Launcher.exe"]));
        Assert.Null(GameExecutables.Pick([null, "", @"C:\G\readme.txt"]));
    }

    [Theory]
    [InlineData(@"C:\Games\Hades\Hades.EXE", "hades.exe")]
    [InlineData("/mnt/c/game.exe", "game.exe")]
    [InlineData("", null)]
    public void File_name_is_lower_case_without_folder(string path, string? expected) =>
        Assert.Equal(expected, GameExecutables.FileName(path));
}

public class GpuSelectionTests
{
    private static readonly GpuAdapter Intel = new("Intel(R) UHD Graphics 770", GpuVendors.IntelPciId, 128UL << 20, false);
    private static readonly GpuAdapter Nvidia = new("NVIDIA GeForce RTX 4070 Laptop GPU", GpuVendors.NvidiaPciId, 8UL << 30, false);
    private static readonly GpuAdapter Warp = new("Microsoft Basic Render Driver", GpuVendors.MicrosoftPciId, 0, true);

    [Theory]
    [InlineData(null, GpuPreference.Default)]
    [InlineData("", GpuPreference.Default)]
    [InlineData("GpuPreference=0;", GpuPreference.Default)]
    [InlineData("GpuPreference=1;", GpuPreference.PowerSaving)]
    [InlineData("SwapEffectUpgradeEnable=1;GpuPreference=2;", GpuPreference.HighPerformance)]
    [InlineData("gpupreference = 2", GpuPreference.HighPerformance)]
    [InlineData("GpuPreference=x;", GpuPreference.Default)]
    public void Windows_preference_is_read_from_the_registry_value(string? value, GpuPreference expected) =>
        Assert.Equal(expected, GpuSelection.ParsePreference(value));

    [Fact]
    public void Hybrid_laptop_uses_the_dedicated_gpu_unless_power_saving_is_chosen()
    {
        GpuAdapter[] adapters = [Intel, Nvidia, Warp];
        Assert.Same(Nvidia, GpuSelection.Choose(adapters, GpuPreference.Default));
        Assert.Same(Nvidia, GpuSelection.Choose(adapters, GpuPreference.HighPerformance));
        Assert.Same(Intel, GpuSelection.Choose(adapters, GpuPreference.PowerSaving));
    }

    [Fact]
    public void Single_gpu_wins_and_software_adapters_never_count()
    {
        var apu = new GpuAdapter("AMD Radeon(TM) 780M", GpuVendors.AmdPciId, 512UL << 20, false);
        Assert.Same(apu, GpuSelection.Choose([apu, Warp], GpuPreference.PowerSaving));
        Assert.Null(GpuSelection.Choose([Warp], GpuPreference.Default));
        Assert.Null(GpuSelection.Choose([], GpuPreference.Default));
    }

    [Fact]
    public void Ties_keep_the_dxgi_order()
    {
        var first = new GpuAdapter("A", GpuVendors.AmdPciId, 1 << 30, false);
        var second = new GpuAdapter("B", GpuVendors.NvidiaPciId, 1 << 30, false);
        Assert.Same(first, GpuSelection.Choose([first, second], GpuPreference.Default));
    }

    [Theory]
    [InlineData(GpuVendors.NvidiaPciId, GpuVendor.Nvidia)]
    [InlineData(GpuVendors.AmdPciId, GpuVendor.Amd)]
    [InlineData(GpuVendors.AmdAltPciId, GpuVendor.Amd)]
    [InlineData(GpuVendors.IntelPciId, GpuVendor.Intel)]
    [InlineData(0x5143u, GpuVendor.Unknown)]
    public void Vendor_comes_from_the_pci_id(uint id, GpuVendor expected) => Assert.Equal(expected, GpuVendors.FromPciId(id));

    [Fact]
    public void Driver_list_is_matched_by_name()
    {
        Assert.Equal(1, GpuSelection.MatchByName(["AMD Radeon 780M", "AMD Radeon RX 7600"], "AMD Radeon RX 7600"));
        Assert.Equal(0, GpuSelection.MatchByName(["Intel(R) Arc(TM) A770 Graphics"], "Intel(R) Arc(TM) A770"));
        Assert.Equal(0, GpuSelection.MatchByName(["Outro nome"], "Placa"));
        Assert.Equal(-1, GpuSelection.MatchByName([], "Placa"));
    }
}

public class DriverLimitStateTests
{
    [Theory]
    [InlineData(false, false, 0u, NvidiaRestoreAction.DeleteProfile)]
    [InlineData(true, false, 0u, NvidiaRestoreAction.DeleteSetting)]
    [InlineData(true, true, 60u, NvidiaRestoreAction.SetValue)]
    public void Nvidia_restore_undoes_exactly_what_the_tako_added(bool existed, bool onProfile, uint value, NvidiaRestoreAction expected)
    {
        var state = new NvidiaLimitState(existed, onProfile, value);
        Assert.Equal(expected, state.RestoreAction);
        Assert.Equal(state, NvidiaLimitState.Parse(state.Serialize()));
    }

    [Fact]
    public void States_round_trip_and_reject_other_vendors()
    {
        var amd = new AmdLimitState(AmdLimitMethod.Chill, true, 30, 72);
        var frtc = new AmdLimitState(AmdLimitMethod.Frtc, false, 0, 150);
        var intel = new IntelLimitState(true, true, 90);
        Assert.Equal(amd, AmdLimitState.Parse(amd.Serialize()));
        Assert.Equal(frtc, AmdLimitState.Parse(frtc.Serialize()));
        Assert.Equal(intel, IntelLimitState.Parse(intel.Serialize()));
        Assert.Null(NvidiaLimitState.Parse(amd.Serialize()));
        Assert.Null(AmdLimitState.Parse("amd;outro;1;0;0"));
        Assert.Null(IntelLimitState.Parse(null));
        Assert.Null(NvidiaLimitState.Parse("nv;2;0;60"));
    }

    [Theory]
    [InlineData(30, 60, true)]
    [InlineData(30, 30, true)]
    [InlineData(90, 60, false)]
    public void Chill_never_puts_the_maximum_below_the_current_minimum(int currentMin, int newMax, bool maxFirst) =>
        Assert.Equal(maxFirst, AmdLimitState.ChillSetsMaxFirst(currentMin, newMax));

    [Fact]
    public void Capabilities_clamp_to_the_driver_range_and_explain_themselves()
    {
        var caps = new FrameRateLimitCapabilities(GpuVendor.Amd, "RX", FrameRateLimitAvailability.Available, 30, 300, false, "Radeon Chill");
        Assert.Equal(0, caps.Clamp(0));
        Assert.Equal(30, caps.Clamp(20));
        Assert.Equal(300, caps.Clamp(500));
        Assert.Equal(144, caps.Clamp(144));
        Assert.Contains("AMD", caps.Explanation, StringComparison.Ordinal);
        Assert.Equal("Indisponível sem permissão de administrador", (caps with { Availability = FrameRateLimitAvailability.NeedsAdmin }).Explanation);
        Assert.False(FrameRateLimitCapabilities.None.IsAvailable);
    }
}

public sealed class DriverFrameRateLimiterTests : IDisposable
{
    private static readonly GpuAdapter Nvidia = new("NVIDIA GeForce RTX 4070", GpuVendors.NvidiaPciId, 12UL << 30, false);
    private static readonly GpuAdapter Radeon = new("AMD Radeon 780M", GpuVendors.AmdPciId, 512UL << 20, false);
    private readonly string _file = Path.Combine(Path.GetTempPath(), $"tako-frl-{Guid.NewGuid():N}.json");

    public void Dispose() => File.Delete(_file);

    [Fact]
    public void Per_app_driver_saves_the_previous_state_once_and_restores_it()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true) { Current = "nv;1;1;60" };
        var limiter = Limiter([Nvidia], nv);

        Assert.True(limiter.Apply(@"C:\Games\Hades\Hades.exe", 45));
        Assert.True(limiter.Apply(@"C:\Games\Hades\Hades.exe", 90)); // a segunda mudança não troca o "antes"
        Assert.Equal(["hades.exe=45", "hades.exe=90"], nv.Applied);
        Assert.Equal(1, nv.Captures);
        Assert.True(File.Exists(_file));

        limiter.Restore();
        Assert.Equal(["hades.exe<-nv;1;1;60"], nv.Restored);
        Assert.False(File.Exists(_file));
    }

    [Fact]
    public void A_crash_is_undone_by_the_next_instance()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true) { Current = "nv;0;0;0" };
        Assert.True(Limiter([Nvidia], nv).Apply(@"C:\G\game.exe", 60));

        var afterCrash = new FakeBackend(GpuVendor.Nvidia, perApp: true);
        Limiter([Nvidia], afterCrash).Restore();
        Assert.Equal(["game.exe<-nv;0;0;0"], afterCrash.Restored);
    }

    [Fact]
    public void No_limit_gives_the_user_value_back_and_does_nothing_without_a_change()
    {
        var amd = new FakeBackend(GpuVendor.Amd, perApp: false) { Current = "amd;chill;0;30;300" };
        var limiter = Limiter([Radeon], amd);

        Assert.True(limiter.Apply(null, 0));
        Assert.Empty(amd.Restored);
        Assert.Equal(0, amd.Captures);

        Assert.True(limiter.Apply(@"C:\G\game.exe", 60));
        Assert.Equal(["(global)=60"], amd.Applied); // global: o exe não importa
        Assert.True(limiter.Apply(@"C:\G\game.exe", 0));
        Assert.Equal(["(global)<-amd;chill;0;30;300"], amd.Restored);
        Assert.False(File.Exists(_file));
    }

    [Fact]
    public void Per_app_driver_without_the_game_exe_waits()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true);
        Assert.False(Limiter([Nvidia], nv).Apply(null, 60));
        Assert.Empty(nv.Applied);
    }

    [Fact]
    public void Nothing_is_written_when_the_previous_value_cannot_be_read()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true) { Current = null };
        Assert.False(Limiter([Nvidia], nv).Apply(@"C:\G\game.exe", 60));
        Assert.Empty(nv.Applied);
        Assert.False(File.Exists(_file));
    }

    [Fact]
    public void Values_are_clamped_to_the_driver_range()
    {
        var amd = new FakeBackend(GpuVendor.Amd, perApp: false) { Current = "amd;frtc;0;0;150", Min = 30, Max = 300 };
        Assert.True(Limiter([Radeon], amd).Apply(null, 1000));
        Assert.Equal(["(global)=300"], amd.Applied);
    }

    [Fact]
    public void Capabilities_follow_the_chosen_gpu_and_missing_drivers()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true);
        var limiter = Limiter([Radeon, Nvidia], nv);
        Assert.Equal(GpuVendor.Nvidia, limiter.Capabilities.Vendor);
        Assert.True(limiter.Capabilities.IsAvailable);

        var amdOnly = Limiter([Radeon], nv); // sem backend AMD
        Assert.Equal(FrameRateLimitAvailability.Unsupported, amdOnly.Capabilities.Availability);
        Assert.False(amdOnly.Apply(null, 60));

        Assert.Equal(FrameRateLimitAvailability.NoGpu, Limiter([], nv).Capabilities.Availability);

        nv.Availability = FrameRateLimitAvailability.NeedsAdmin;
        Assert.Equal(FrameRateLimitAvailability.NeedsAdmin, limiter.Capabilities.Availability);
        Assert.False(limiter.Apply(@"C:\G\game.exe", 60));
    }

    [Fact]
    public void Power_saving_preference_moves_the_limit_to_the_integrated_gpu()
    {
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true) { Current = "nv;1;0;0" };
        var amd = new FakeBackend(GpuVendor.Amd, perApp: false) { Current = "amd;frtc;0;0;150" };
        var limiter = new DriverFrameRateLimiter(() => [Radeon, Nvidia], exe => exe!.Contains("light", StringComparison.Ordinal) ? GpuPreference.PowerSaving : GpuPreference.Default, [nv, amd], new FrameRateLimitBackupStore(_file));

        Assert.True(limiter.Apply(@"C:\G\light.exe", 30));
        Assert.True(limiter.Apply(@"C:\G\heavy.exe", 60));
        Assert.Equal(["(global)=30"], amd.Applied);
        Assert.Equal(["heavy.exe=60"], nv.Applied);

        limiter.Restore();
        Assert.Single(amd.Restored);
        Assert.Single(nv.Restored);
    }

    [Fact]
    public void Unreadable_backup_file_means_nothing_to_restore()
    {
        File.WriteAllText(_file, "{ não é json");
        var nv = new FakeBackend(GpuVendor.Nvidia, perApp: true);
        Limiter([Nvidia], nv).Restore();
        Assert.Empty(nv.Restored);
    }

    [Fact]
    public void Null_limiter_never_does_anything()
    {
        var limiter = new NullFrameRateLimiter();
        Assert.False(limiter.Capabilities.IsAvailable);
        Assert.False(limiter.Apply("x.exe", 60));
        Assert.Null(limiter.ReadCurrent("x.exe"));
        limiter.Restore();
    }

    private DriverFrameRateLimiter Limiter(IReadOnlyList<GpuAdapter> adapters, params IDriverFrameRateBackend[] backends) =>
        new(() => adapters, _ => GpuPreference.Default, backends, new FrameRateLimitBackupStore(_file));

    private sealed class FakeBackend(GpuVendor vendor, bool perApp) : IDriverFrameRateBackend
    {
        public GpuVendor Vendor { get; } = vendor;

        public FrameRateLimitAvailability Availability { get; set; } = FrameRateLimitAvailability.Available;

        public string? Current { get; set; } = "x";

        public int Min { get; init; } = 1;

        public int Max { get; init; } = 1000;

        public int Captures { get; private set; }

        public List<string> Applied { get; } = [];

        public List<string> Restored { get; } = [];

        public DriverProbe Probe(GpuAdapter adapter) => new(Availability, Min, Max, perApp, "fake");

        public int? Read(GpuAdapter adapter, string? exeName) => 0;

        public string? Capture(GpuAdapter adapter, string? exeName)
        {
            Captures++;
            return Current;
        }

        public bool Apply(GpuAdapter adapter, string? exeName, int fps)
        {
            Applied.Add($"{exeName ?? "(global)"}={fps}");
            return true;
        }

        public bool Restore(GpuAdapter adapter, string? exeName, string state)
        {
            Restored.Add($"{exeName ?? "(global)"}<-{state}");
            return true;
        }
    }
}
