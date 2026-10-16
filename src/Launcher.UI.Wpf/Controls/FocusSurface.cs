using System.Windows;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Diz aos controles dentro de um elemento que ele está pintado com a superfície sólida (<c>surface-solid</c>, branca):
/// linha de painel focada, categoria focada, item selecionado da barra lateral, tecla focada, pílula primária. O valor é
/// herdado pela árvore, então a chave, o marcador, o valor, a seta e os ícones trocam para as cores "OnSolid" sem que
/// cada tela repita o gatilho (DESIGN.md › Foco; antes a chave branca sumia na linha branca).
/// </summary>
public static class FocusSurface
{
    public static readonly DependencyProperty IsSolidProperty = DependencyProperty.RegisterAttached(
        "IsSolid", typeof(bool), typeof(FocusSurface), new FrameworkPropertyMetadata(false, FrameworkPropertyMetadataOptions.Inherits));

    public static bool GetIsSolid(DependencyObject d) => (bool)d.GetValue(IsSolidProperty);

    public static void SetIsSolid(DependencyObject d, bool value) => d.SetValue(IsSolidProperty, value);
}
