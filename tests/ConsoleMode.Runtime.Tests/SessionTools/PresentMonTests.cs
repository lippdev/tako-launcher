using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class PresentMonArgumentsTests
{
    [Fact]
    public void Capture_goes_to_stdout_in_a_session_named_for_tako()
    {
        var args = PresentMonArguments.Build();
        Assert.Contains("--output_stdout", args);
        int name = args.ToList().IndexOf("--session_name");
        Assert.True(name >= 0);
        Assert.Equal("Tako_FPS", args[name + 1]);
        Assert.NotEqual("PresentMon", args[name + 1]); // o nome padrão é o do CapFrameX e de outros
        Assert.Contains("--stop_existing_session", args); // só a sessão com o nome do Tako
        Assert.DoesNotContain("--terminate_existing_session", args);
        Assert.DoesNotContain("--restart_as_admin", args);
    }

    [Fact]
    public void Excludes_get_exe_and_skip_names_that_would_split()
    {
        var args = PresentMonArguments.Build(["Tako", "dwm.exe", "My Game.exe", "", "a\"b"]);
        Assert.Equal(["--exclude", "Tako.exe", "--exclude", "dwm.exe"], args.SkipWhile(a => a != "--exclude"));
    }
}

public class PresentMonReaderTests
{
    // Cabeçalho real do PresentMon 2.6.0 (métricas padrão) e duas linhas de um jogo DXGI.
    private const string Header2x = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,CPUStartTime,FrameTime,CPUBusy,CPUWait,GPULatency,GPUTime,GPUBusy,GPUWait,DisplayLatency,DisplayedTime,AnimationError,AnimationTime,MsFlipDelay";

    // Cabeçalho do PresentMon 1.x / --v1_metrics.
    private const string Header1x = "Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,msInPresentAPI,msBetweenPresents";

    [Fact]
    public void Reads_frame_time_by_column_name_in_version_2()
    {
        var reader = new PresentMonReader();
        Assert.False(reader.TryRead("warning: --no_console_stats added because stdout does not support statistics reporting.", out _));
        Assert.Null(reader.Columns);
        Assert.False(reader.TryRead(Header2x, out _));
        Assert.NotNull(reader.Columns);

        Assert.True(reader.TryRead("Cyberpunk2077.exe,4242,0x0000021F4C8D3A10,DXGI,0,0,1,Hardware: Independent Flip,12.5,16.667,10.1,6.5,1.2,8.3,8.0,0.3,20.4,16.6,NA,NA,NA", out var sample));
        Assert.Equal(new PresentSample(4242, "Cyberpunk2077.exe", "DXGI", 16.667), sample);
    }

    [Fact]
    public void Reads_the_v1_column_names_too()
    {
        var reader = new PresentMonReader();
        reader.TryRead(Header1x, out _);
        Assert.True(reader.TryRead("game.exe,77,0x1,D3D9,1,0,0,3.5,0.12,8.33", out var sample));
        Assert.Equal(77, sample.ProcessId);
        Assert.Equal("D3D9", sample.Runtime);
        Assert.Equal(8.33, sample.FrameTimeMs, 3);
    }

    [Theory]
    [InlineData("<error>,4242,0x1,DXGI,0,0,1,Composed: Flip,12.5,16.6,1,1,1,1,1,1,1,1,1,1,1")] // erro do PresentMon
    [InlineData("game.exe,abc,0x1,DXGI,0,0,1,Composed: Flip,12.5,16.6,1,1,1,1,1,1,1,1,1,1,1")] // pid ruim
    [InlineData("game.exe,10,0x1,DXGI,0,0,1,Composed: Flip,12.5,NA,1,1,1,1,1,1,1,1,1,1,1")] // sem tempo de quadro
    [InlineData("game.exe,10,0x1,DXGI,0,0,1,Composed: Flip,12.5,0,1,1,1,1,1,1,1,1,1,1,1")] // zero
    [InlineData("game.exe,10,0x1,DXGI,0,0,1,Composed: Flip,12.5,60000,1,1,1,1,1,1,1,1,1,1,1")] // pausa, não quadro
    [InlineData("my,game.exe,10,0x1,DXGI,0,0,1,Composed: Flip,12.5,16.6,1,1,1,1,1,1,1,1,1,1,1")] // vírgula no nome
    [InlineData("game.exe,10,0x1")] // linha cortada
    [InlineData("")]
    public void Skips_lines_it_cannot_trust(string line)
    {
        var reader = new PresentMonReader();
        reader.TryRead(Header2x, out _);
        Assert.False(reader.TryRead(line, out _));
    }

    [Fact]
    public void Numbers_are_read_the_same_in_any_windows_language()
    {
        var culture = System.Globalization.CultureInfo.CurrentCulture;
        try
        {
            System.Globalization.CultureInfo.CurrentCulture = new System.Globalization.CultureInfo("pt-BR");
            var reader = new PresentMonReader();
            reader.TryRead(Header2x, out _);
            Assert.True(reader.TryRead("g.exe,5,0x1,DXGI,0,0,1,Composed: Flip,12.5,6.944,1,1,1,1,1,1,1,1,1,1,1", out var sample));
            Assert.Equal(6.944, sample.FrameTimeMs, 3);
        }
        finally
        {
            System.Globalization.CultureInfo.CurrentCulture = culture;
        }
    }

    [Fact]
    public void A_header_without_frame_time_is_not_a_header()
    {
        Assert.Null(PresentMonColumns.FromHeader("Application,ProcessID,SwapChainAddress"));
        Assert.Null(PresentMonColumns.FromHeader("PresentMon 2.6.0"));
        Assert.Null(PresentMonColumns.FromHeader(null));
    }
}
