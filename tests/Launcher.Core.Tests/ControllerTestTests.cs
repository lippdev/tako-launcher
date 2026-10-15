using Launcher.Core.Diagnostics;
using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class ControllerTestTests
{
    [Fact]
    public void Hex_is_spaced_uppercase_and_cut()
    {
        Assert.Equal("31 00 7F", ControllerTest.Hex([0x31, 0x00, 0x7F]));
        Assert.Equal("01 02 …", ControllerTest.Hex([1, 2, 3], max: 2));
        Assert.Equal(string.Empty, ControllerTest.Hex([]));
    }

    [Fact]
    public void Describes_dpad_buttons_and_stick()
    {
        string text = ControllerTest.Describe(new GamepadSnapshot(GamepadButtons.DPadUp | GamepadButtons.A, 0, 0.95));

        Assert.Contains("direcional: cima", text, StringComparison.Ordinal);
        Assert.Contains("A/✕", text, StringComparison.Ordinal);
        Assert.Contains("y +0.95", text, StringComparison.Ordinal);
        Assert.Contains("direcional: solto", ControllerTest.Describe(default(GamepadSnapshot)), StringComparison.Ordinal);
    }

    [Fact]
    public void Describes_edge_extras_and_raw_hat()
    {
        var reading = new SonyHidReading(new GamepadSnapshot(GamepadButtons.None, 0, 0), SonyReportLayout.DualSenseUsb, 8, 0, 0, 0, 255, SonyExtraButtons.LeftPaddle | SonyExtraButtons.RightFunction);
        string text = ControllerTest.Describe(reading);

        Assert.Contains("chapéu 8", text, StringComparison.Ordinal);
        Assert.Contains("R2 255", text, StringComparison.Ordinal);
        Assert.Contains("Fn dir.", text, StringComparison.Ordinal);
        Assert.Contains("traseiro esq.", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Lines_show_source_model_connection_and_ids_without_device_path()
    {
        var probe = new ControllerProbe("HID", "DualSense Edge", HidTransport.Bluetooth, 0x054C, 0x0DF2, ControllerTest.LayoutLabel(SonyReportLayout.Simple, 0x01, 78), "direcional: solto", ["agora: 01 80 80"]);
        var lines = ControllerTest.Lines([probe], steamRunning: false);

        Assert.Equal("Steam em execução: não", lines[0]);
        Assert.Equal("HID · DualSense Edge · Bluetooth · 054C:0DF2 · relatório 0x01 simples, 78 bytes", lines[1]);
        Assert.Equal("  direcional: solto", lines[2]);
        Assert.Equal("  agora: 01 80 80", lines[3]);
        Assert.Equal(["Steam em execução: ?", "nenhum controle lido agora"], ControllerTest.Lines([], null));
    }

    [Fact]
    public void Diagnostics_report_includes_the_controller_test_section()
    {
        var snapshot = new SystemSnapshot
        {
            GeneratedAt = new DateTimeOffset(2026, 10, 15, 14, 30, 5, TimeSpan.Zero),
            TakoVersion = "0.1.0-alpha.10",
            InstallKind = "instalada",
            Architecture = "X64",
            Runtime = ".NET 10.0.0",
            ControllerTest = ["Steam em execução: sim", "HID · DualSense Edge · USB · 054C:0DF2"],
        };

        string report = SystemReport.Format(snapshot, LogRedactor.Default);

        Assert.Contains("[Teste de controle]\n  Steam em execução: sim\n  HID · DualSense Edge · USB · 054C:0DF2\n", report, StringComparison.Ordinal);
        Assert.DoesNotContain("[Teste de controle]", SystemReport.Format(snapshot with { ControllerTest = [] }, LogRedactor.Default), StringComparison.Ordinal);
    }
}
