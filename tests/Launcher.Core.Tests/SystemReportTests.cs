using Launcher.Core.Diagnostics;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class SystemReportTests
{
    private static SystemSnapshot Sample() => new()
    {
        GeneratedAt = new DateTimeOffset(2026, 10, 15, 14, 30, 5, TimeSpan.FromHours(-3)),
        TakoVersion = "0.1.0-alpha.7",
        InstallKind = "instalada",
        DataFolder = @"C:\Users\Fulano\AppData\Local\Tako",
        WindowsName = "Windows 11 Pro",
        WindowsDisplayVersion = "24H2",
        WindowsBuild = "26100.4351",
        Architecture = "X64",
        Runtime = ".NET 10.0.0",
        Cpu = "AMD Ryzen 7 7840U",
        LogicalProcessors = 16,
        MemoryBytes = 16L * 1024 * 1024 * 1024,
        Displays =
        [
            new(@"\\.\DISPLAY1", "AMD Radeon 780M", 1920, 1080, 125, 120, IsPrimary: true),
            new(@"\\.\DISPLAY2", null, 3440, 1440, null, null, IsPrimary: false),
        ],
        Controllers = ["Xbox (XInput, posição 1)", "DualSense (HID)"],
        ActiveController = "Controle Xbox",
        SteamFound = true,
        SteamGames = 42,
        ManualGames = 3,
        TotalGames = 45,
        XboxModeLabel = "Indeterminado",
        XboxModeReason = "Em desktops e notebooks a Microsoft libera o Xbox mode aos poucos",
        XboxModeSignals = "build 26100, app Xbox: sim, DeviceForm: 0",
        SafeMode = false,
        Theme = "Padrão",
    };

    [Fact]
    public void Report_lists_every_section_of_the_hardware_checklist()
    {
        string report = SystemReport.Format(Sample(), new LogRedactor(@"C:\Users\Fulano"));

        Assert.Contains("0.1.0-alpha.7", report, StringComparison.Ordinal);
        Assert.Contains("instalada", report, StringComparison.Ordinal);
        Assert.Contains("Windows 11 Pro 24H2", report, StringComparison.Ordinal);
        Assert.Contains("26100.4351", report, StringComparison.Ordinal);
        Assert.Contains(@"\\.\DISPLAY1 (principal): 1920x1080 16:9, escala 125%, 120 Hz · AMD Radeon 780M", report, StringComparison.Ordinal);
        Assert.Contains("3440x1440 21:9, escala ?, ? Hz", report, StringComparison.Ordinal);
        Assert.Contains("DualSense (HID)", report, StringComparison.Ordinal);
        Assert.Contains("encontrada, 42 jogos", report, StringComparison.Ordinal);
        Assert.Contains("Indeterminado", report, StringComparison.Ordinal);
        Assert.Contains("DeviceForm: 0", report, StringComparison.Ordinal);
        Assert.Contains("desligado", report, StringComparison.Ordinal);
        Assert.Contains("16.0 GB", report, StringComparison.Ordinal);
    }

    [Fact]
    public void Report_has_no_personal_data()
    {
        var snapshot = Sample() with { SafeMode = true, SafeModeDetail = @"tema em C:\Users\Fulano\Desktop\x falhou no PC-DO-FULANO" };
        string report = SystemReport.Format(snapshot, new LogRedactor(@"C:\Users\Fulano"), userName: "Fulano", machineName: "PC-DO-FULANO");

        Assert.DoesNotContain("Fulano", report, StringComparison.OrdinalIgnoreCase);
        Assert.Contains(@"%USERPROFILE%\AppData\Local\Tako", report, StringComparison.Ordinal);
        Assert.Contains(SystemReport.MachineMarker, report, StringComparison.Ordinal);
    }

    [Fact]
    public void Short_names_are_replaced_only_as_whole_words()
    {
        var snapshot = Sample() with { Cpu = "Banana CPU", Theme = "Ana" };
        string report = SystemReport.Format(snapshot, new LogRedactor(null), userName: "ana");

        Assert.Contains("Banana CPU", report, StringComparison.Ordinal);
        Assert.Contains("Tema:                  " + SystemReport.UserMarker, report, StringComparison.Ordinal);
    }

    [Fact]
    public void Missing_values_are_reported_instead_of_hidden()
    {
        var snapshot = Sample() with { Displays = [], Controllers = [], Cpu = null, MemoryBytes = null, SteamFound = false, ActiveController = null };
        string report = SystemReport.Format(snapshot, new LogRedactor(null));

        Assert.Contains("nenhum monitor lido", report, StringComparison.Ordinal);
        Assert.Contains("nenhum controle detectado", report, StringComparison.Ordinal);
        Assert.Contains("não encontrada", report, StringComparison.Ordinal);
        Assert.Contains("Processador:           não informado", report, StringComparison.Ordinal);
    }

    [Theory]
    [InlineData(2560, 1080, "21:9")]
    [InlineData(5120, 1440, "32:9")]
    [InlineData(1280, 800, "16:10")]
    [InlineData(3840, 2160, "16:9")]
    [InlineData(1000, 1000, "1.00:1")]
    public void Aspect_ratio_matches_the_checklist_labels(int width, int height, string expected) =>
        Assert.Equal(expected, new DisplayInfo("d", null, width, height, 100, 60, true).AspectRatio);

    [Fact]
    public void File_name_is_sortable() =>
        Assert.Equal("diagnostico-20261015-143005.txt", SystemReport.FileName(Sample().GeneratedAt));
}
