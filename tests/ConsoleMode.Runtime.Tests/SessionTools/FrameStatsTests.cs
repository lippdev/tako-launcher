using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class FrameMathTests
{
    [Fact]
    public void Average_fps_is_frames_over_summed_time()
    {
        // 3 quadros em 50 ms = 60 FPS (a média dos FPS de cada quadro daria ~73).
        Assert.Equal(60, FrameMath.AverageFps([10, 10, 30]), 6);
        Assert.True(double.IsNaN(FrameMath.AverageFps([])));
    }

    [Theory]
    [InlineData(0.0, 1)]
    [InlineData(1.0, 10)]
    [InlineData(0.5, 5.5)]
    public void Quantile_matches_r8(double p, double expected)
    {
        double[] values = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
        Assert.Equal(expected, FrameMath.Quantile(values, p), 6);
    }

    [Fact]
    public void Quantile_r8_interpolates_like_mathnet()
    {
        // R-8 de 1..10 em p = 0,9: h = (10 + 1/3)·0,9 + 1/3 = 9,6333 → 9 + 0,6333·(10 − 9).
        double[] values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        Assert.Equal(9.633333, FrameMath.Quantile(values, 0.9), 5);
    }

    [Fact]
    public void One_percent_low_is_the_fps_of_the_slowest_frames()
    {
        // 990 quadros de 10 ms (100 FPS) e 10 travadas de 50 ms: o 1% low fica em 20 FPS.
        var frames = Enumerable.Repeat(10.0, 990).Concat(Enumerable.Repeat(50.0, 10)).ToList();
        Assert.Equal(20, FrameMath.OnePercentLow(frames), 1);
        Assert.Equal(100, FrameMath.OnePercentLow(Enumerable.Repeat(10.0, 500).ToList()), 6);
    }
}

public class FrameStatsWindowTests
{
    private static PresentSample Frame(int pid, double ms, string runtime = "DXGI") => new(pid, "game.exe", runtime, ms);

    [Fact]
    public void Only_frames_of_the_game_processes_count()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        Assert.False(window.Add(Frame(99, 5), TimeSpan.Zero)); // navegador, Discord...
        Assert.True(window.Add(Frame(10, 16.667), TimeSpan.Zero));
        var stats = window.Snapshot(TimeSpan.Zero)!;
        Assert.Equal(10, stats.ProcessId);
        Assert.Equal(60, stats.Fps, 0);
    }

    [Fact]
    public void Fps_covers_about_the_last_second_of_frames_even_when_lines_arrive_in_bursts()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        var now = TimeSpan.FromSeconds(5);
        // 2 s a 30 FPS e depois 1 s a 120 FPS, tudo entregue de uma vez.
        for (int i = 0; i < 60; i++)
        {
            window.Add(Frame(10, 33.333), now);
        }

        for (int i = 0; i < 120; i++)
        {
            window.Add(Frame(10, 8.333), now);
        }

        var stats = window.Snapshot(now)!;
        Assert.InRange(stats.Fps, 115, 121); // o último segundo, não a média das rajadas
        Assert.InRange(stats.FrameTimeMs, 8.3, 8.7);
        Assert.NotNull(stats.OnePercentLow);
        Assert.Equal(30, stats.OnePercentLow!.Value, 0); // o trecho lento está nos últimos 10 s
    }

    [Fact]
    public void One_percent_low_waits_for_enough_frames()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        for (int i = 0; i < FrameStatsWindow.MinFramesForLow - 1; i++)
        {
            window.Add(Frame(10, 100), TimeSpan.Zero);
        }

        Assert.Null(window.Snapshot(TimeSpan.Zero)!.OnePercentLow);
    }

    [Fact]
    public void A_game_that_stops_presenting_shows_nothing()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        window.Add(Frame(10, 16), TimeSpan.FromSeconds(1));
        Assert.NotNull(window.Snapshot(TimeSpan.FromSeconds(2)));
        Assert.Null(window.Snapshot(TimeSpan.FromSeconds(1) + FrameStatsWindow.StaleAfter + TimeSpan.FromMilliseconds(1)));
    }

    [Fact]
    public void With_launcher_and_game_presenting_the_busier_one_wins()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10, 20]);
        for (int i = 0; i < 30; i++)
        {
            window.Add(Frame(10, 33.3, "D3D11"), TimeSpan.Zero); // launcher a 30 FPS
        }

        for (int i = 0; i < 144; i++)
        {
            window.Add(Frame(20, 6.944, "DXGI"), TimeSpan.Zero); // jogo a 144 FPS
        }

        var stats = window.Snapshot(TimeSpan.Zero)!;
        Assert.Equal(20, stats.ProcessId);
        Assert.Equal("DXGI", stats.Runtime);
        Assert.Equal(144, stats.Fps, 0);
    }

    [Fact]
    public void Processes_that_leave_the_game_are_dropped()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        window.Add(Frame(10, 16), TimeSpan.Zero);
        window.SetGameProcesses([11]); // o launcher fechou e abriu o jogo
        Assert.Null(window.Snapshot(TimeSpan.Zero));
        window.Clear();
        Assert.False(window.Add(Frame(11, 16), TimeSpan.Zero));
    }

    [Fact]
    public void Old_frames_leave_the_window()
    {
        var window = new FrameStatsWindow();
        window.SetGameProcesses([10]);
        for (int i = 0; i < 2000; i++)
        {
            window.Add(Frame(10, 100), TimeSpan.Zero); // 200 s de jogo a 10 FPS
        }

        for (int i = 0; i < 60; i++)
        {
            window.Add(Frame(10, 16.667), TimeSpan.Zero);
        }

        // O 1% low olha só os últimos ~10 s: 1 s a 60 FPS + 9 s a 10 FPS.
        var stats = window.Snapshot(TimeSpan.Zero)!;
        Assert.Equal(60, stats.Fps, 0);
        Assert.Equal(10, stats.OnePercentLow!.Value, 0);
    }
}
