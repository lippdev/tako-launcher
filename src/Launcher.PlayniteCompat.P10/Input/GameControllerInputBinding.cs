using System.Windows;
using System.Windows.Input;

namespace Launcher.PlayniteCompat.P10.Input;

/// <summary>
/// <c>pin:GameControllerInputBinding</c> do Playnite (<c>Playnite.Input</c>), liberado por nome (MVP-018). No Playnite
/// liga um botão do controle a um comando; no Tako a entrada do controle é do launcher (navegação, Guia, Menu), então o
/// atalho do tema fica <b>inerte</b>: o gesto nunca corresponde a nenhuma entrada e o comando nunca é executado. Existe
/// para o XAML do tema carregar.
/// </summary>
public sealed class GameControllerInputBinding : InputBinding
{
    public static readonly DependencyProperty ButtonProperty =
        DependencyProperty.Register(nameof(Button), typeof(string), typeof(GameControllerInputBinding), new PropertyMetadata(null));

    public GameControllerInputBinding()
    {
        Gesture = new NeverGesture();
    }

    /// <summary>Botão do controle no Playnite (A, B, X, Y, Start...). Só informativo.</summary>
    public string? Button
    {
        get => (string?)GetValue(ButtonProperty);
        set => SetValue(ButtonProperty, value);
    }

    protected override Freezable CreateInstanceCore() => new GameControllerInputBinding();

    /// <summary>Gesto que não corresponde a nenhuma entrada.</summary>
    private sealed class NeverGesture : InputGesture
    {
        public override bool Matches(object targetElement, InputEventArgs inputEventArgs) => false;
    }
}
