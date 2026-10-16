using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.Services;

/// <summary>Configurações › Capas e metadados › Capas animadas (ADR-0023): só a capa em foco por padrão.</summary>
public sealed partial class SettingsService
{
    [ObservableProperty]
    private AnimatedCoversMode _animatedCovers = AnimatedCoversMode.FocusedOnly;

    partial void OnAnimatedCoversChanged(AnimatedCoversMode value) => Save(nameof(AnimatedCovers), value.ToString());

    private void LoadCovers(IReadOnlyDictionary<string, string> saved) =>
        AnimatedCovers = AnimationPolicy.Parse(saved.GetValueOrDefault(nameof(AnimatedCovers)));
}
