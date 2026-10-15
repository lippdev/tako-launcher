using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Painel Áudio da Central de Controle (MVP-002): volume, mudo e saída de áudio de verdade, pelo
/// <see cref="ShellViewModel.AudioVolume"/>. ← → no volume anda de 5 em 5% (e tira do mudo, como o botão de volume);
/// A no mudo liga/desliga sem fechar o painel; A numa saída troca a saída padrão e fecha.
/// </summary>
public sealed partial class ControlCenterViewModel
{
    private AudioVolumeControl Audio => _shell.AudioVolume;

    private IReadOnlyList<PanelSection> BuildAudioSections()
    {
        AudioOutputLevel? level;
        IReadOnlyList<AudioDevice> outputs;
        try
        {
            level = Audio.Current();
            outputs = Audio.Outputs();
        }
        catch (RuntimeEnvironmentException)
        {
            return [new("Áudio", "Não foi possível ler o áudio do Windows agora.", [new PanelRowViewModel { Label = "Fechar", Command = new RelayCommand(() => ClosePopover()) }])];
        }

        if (level is null)
        {
            return [new("Áudio", "Nenhuma saída de áudio ativa no Windows.", [new PanelRowViewModel { Label = "Fechar", Command = new RelayCommand(() => ClosePopover()) }])];
        }

        PanelRowViewModel? slider = null;
        PanelRowViewModel? mute = null;
        void Show(AudioOutputLevel? now)
        {
            if (now is null)
            {
                return;
            }

            slider!.Fraction = now.VolumePercent / 100d;
            slider.EndText = now.Muted ? "Mudo" : $"{now.VolumePercent}%";
            mute!.IsOn = now.Muted;
            UpdateAudioCard(now);
        }

        void Apply(Func<AudioOutputLevel?> change)
        {
            try
            {
                Show(change());
            }
            catch (RuntimeEnvironmentException)
            {
                _shell.Toast("Não foi possível ajustar o volume");
            }
        }

        slider = new PanelRowViewModel
        {
            Key = "audio:volume",
            Label = "Volume",
            IconKey = "Icon.Volume",
            IsSlider = true,
            Command = new RelayCommand(() => { }),
            AdjustCommand = new RelayCommand<object?>(delta => Apply(() => Audio.StepVolume(delta is int d ? d : 0))),
            SetFractionCommand = new RelayCommand<double>(fraction =>
                Apply(() => Audio.SetVolumePercent((int)Math.Round(Math.Clamp(fraction, 0, 1) * 100 / AudioVolumeControl.Step) * AudioVolumeControl.Step))),
        };
        mute = new PanelRowViewModel
        {
            Key = "audio:mute",
            Label = "Mudo",
            IsToggle = true,
            Command = new RelayCommand(() => Apply(Audio.ToggleMute)),
        };
        Show(level);

        var outputRows = outputs.Select(device => new PanelRowViewModel
        {
            Key = $"audio:output:{device.Id}",
            Label = device.Name,
            IsRadio = true,
            IsOn = string.Equals(device.Id, level.DeviceId, StringComparison.OrdinalIgnoreCase),
            Command = new RelayCommand(() => ChooseOutput(device)),
        }).ToList();

        return
        [
            new("Volume", level.Name, [slider, mute]),
            new("Saída de áudio", Audio.CanChangeOutput ? "Ao encerrar o jogo, a saída de antes volta." : "Este Windows não deixa o Tako trocar a saída de áudio.", outputRows),
        ];
    }

    private void ChooseOutput(AudioDevice device)
    {
        if (!Audio.CanChangeOutput)
        {
            _shell.Toast("Este Windows não deixa o Tako trocar a saída de áudio");
            return;
        }

        try
        {
            Audio.SetOutput(device.Id);
            Choose($"Áudio saindo por: {device.Name}");
        }
        catch (RuntimeEnvironmentException)
        {
            _shell.Toast($"Não foi possível trocar para {device.Name}");
        }
    }

    /// <summary>Cartão de áudio: volume e saída reais (lidos de novo quando nada é passado).</summary>
    private void UpdateAudioCard(AudioOutputLevel? level = null)
    {
        var card = Cards[3];
        try
        {
            level ??= Audio.Current();
        }
        catch (RuntimeEnvironmentException)
        {
            level = null;
        }

        card.Caption = level is null ? "Áudio" : $"Volume · {level.Name}";
        card.Value = level is null ? "Sem saída" : level.Muted ? "Mudo" : $"{level.VolumePercent}%";
        card.Meter = level is null || level.Muted ? 0 : level.VolumePercent / 100d;
    }
}
