using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Simulation;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public class AudioOutputTests
{
    private static readonly AudioOutputChoice Headset = new("sim-headset", "Fone (simulado)");

    private static SimulatedAudioEndpoints WithTv() => new(SimulatedAudioEndpoints.Default with
    {
        Devices = [.. SimulatedAudioEndpoints.Default.Devices, new AudioDevice("sim-tv", "LG TV (NVIDIA High Definition Audio)")],
    });

    [Fact]
    public void Switch_makes_the_choice_default_in_every_role_and_restore_brings_back_the_previous_one()
    {
        var audio = new SimulatedAudioEndpoints();
        var sound = new AudioOutputSwitch(audio);

        var (outcome, record) = sound.Apply(Headset);

        Assert.Equal(AudioSwitchOutcome.Switched, outcome);
        Assert.All(Enum.GetValues<AudioRole>(), role => Assert.Equal("sim-headset", audio.Current.DefaultFor(role)));
        Assert.Equal(3, sound.Restore(record!));
        Assert.True(audio.Current.IsEquivalentTo(SimulatedAudioEndpoints.Default));
    }

    [Fact]
    public void Restore_keeps_a_choice_the_user_made_during_the_game()
    {
        var audio = WithTv();
        var sound = new AudioOutputSwitch(audio);
        var (_, record) = sound.Apply(Headset);
        audio.SetDefaultDevice("sim-tv", AudioRole.Multimedia); // trocou na Central de Controle

        Assert.Equal(2, sound.Restore(record!));
        Assert.Equal("sim-speakers", audio.Current.DefaultFor(AudioRole.Console));
        Assert.Equal("sim-tv", audio.Current.DefaultFor(AudioRole.Multimedia));
    }

    [Fact]
    public void Restore_skips_a_previous_device_that_was_disconnected()
    {
        var audio = new SimulatedAudioEndpoints();
        var sound = new AudioOutputSwitch(audio);
        var (_, record) = sound.Apply(Headset);
        audio.Current = audio.Current with { Devices = [audio.Current.Devices[1]] }; // alto-falantes sumiram

        Assert.Equal(0, sound.Restore(record!));
        Assert.Equal("sim-headset", audio.Current.DefaultFor(AudioRole.Console));
    }

    [Fact]
    public void Nothing_changes_when_the_choice_is_missing_already_default_or_unsupported()
    {
        var audio = new SimulatedAudioEndpoints();
        var sound = new AudioOutputSwitch(audio);

        Assert.Equal(AudioSwitchOutcome.DeviceMissing, sound.Apply(new AudioOutputChoice("sim-tv", "TV")).Outcome);
        Assert.Equal(AudioSwitchOutcome.AlreadyDefault, sound.Apply(new AudioOutputChoice("sim-speakers", "Alto-falantes")).Outcome);
        audio.CanSetDefaultDevice = false;
        Assert.Equal(AudioSwitchOutcome.Unsupported, sound.Apply(Headset).Outcome);
        Assert.DoesNotContain(audio.Calls, c => c.StartsWith("default", StringComparison.Ordinal));
    }

    [Fact]
    public void A_choice_is_found_by_id_then_by_a_unique_name()
    {
        var state = WithTv().Current;

        Assert.Equal("sim-tv", new AudioOutputChoice("sim-tv", "outro nome").FindIn(state)?.Id);
        Assert.Equal("sim-tv", new AudioOutputChoice("id-antigo", "LG TV (NVIDIA High Definition Audio)").FindIn(state)?.Id);
        Assert.Null(new AudioOutputChoice("id-antigo", "Outra TV").FindIn(state));

        var twins = state with { Devices = [.. state.Devices, new AudioDevice("sim-tv-2", "LG TV (NVIDIA High Definition Audio)")] };
        Assert.Null(new AudioOutputChoice("id-antigo", "LG TV (NVIDIA High Definition Audio)").FindIn(twins)); // seria um palpite
    }

    [Fact]
    public void Volume_control_reads_and_adjusts_the_default_output()
    {
        var audio = new SimulatedAudioEndpoints();
        var volume = new AudioVolumeControl(audio);

        Assert.Equal(new AudioOutputLevel("sim-speakers", "Alto-falantes (simulado)", 70, false), volume.Current());
        Assert.Equal(75, volume.StepVolume(1)!.VolumePercent);
        Assert.Equal(0, volume.SetVolumePercent(-20)!.VolumePercent);
        Assert.Equal(100, volume.SetVolumePercent(140)!.VolumePercent);
        Assert.True(volume.ToggleMute()!.Muted);
        Assert.True(volume.Current()!.Muted);

        var unmuted = volume.StepVolume(-1)!; // mexer no volume tira do mudo, como girar o botão
        Assert.Equal(95, unmuted.VolumePercent);
        Assert.False(volume.Current()!.Muted);
    }

    [Fact]
    public void Volume_control_switches_the_output_and_handles_no_output()
    {
        var audio = new SimulatedAudioEndpoints();
        var volume = new AudioVolumeControl(audio);

        volume.SetOutput("sim-headset");
        Assert.Equal("sim-headset", volume.Current()!.DeviceId);
        Assert.Equal(50, volume.Current()!.VolumePercent);

        audio.Current = AudioState.Empty;
        Assert.Null(volume.Current());
        Assert.Null(volume.SetVolumePercent(10));
        Assert.Null(volume.ToggleMute());
    }
}
