using System.Windows;
using System.Windows.Input;

namespace Launcher.UI.Wpf.Navigation;

/// <summary>
/// Propriedades anexadas da navegação por controle (MVP-028/029). Um elemento com <c>Nav.Group</c> é navegável;
/// o <see cref="FocusNavigator"/> decide o vizinho pela posição na tela.
/// </summary>
public static class Nav
{
    /// <summary>Grupo do elemento (fileira, barra lateral...). Presença = navegável.</summary>
    public static readonly DependencyProperty GroupProperty =
        DependencyProperty.RegisterAttached("Group", typeof(string), typeof(Nav), new PropertyMetadata(null));

    /// <summary>Identificador estável para restaurar o foco depois de voltar.</summary>
    public static readonly DependencyProperty KeyProperty =
        DependencyProperty.RegisterAttached("Key", typeof(string), typeof(Nav), new PropertyMetadata(null));

    /// <summary>Recebe o foco ao abrir a tela ou a sobreposição.</summary>
    public static readonly DependencyProperty IsDefaultProperty =
        DependencyProperty.RegisterAttached("IsDefault", typeof(bool), typeof(Nav), new PropertyMetadata(false));

    /// <summary>← → executam este comando com -1/+1 em vez de mover o foco (ex.: barra de volume).</summary>
    public static readonly DependencyProperty AdjustCommandProperty =
        DependencyProperty.RegisterAttached("AdjustCommand", typeof(ICommand), typeof(Nav), new PropertyMetadata(null));

    /// <summary>Container cujo conteúdo rola para manter o foco visível: "Horizontal" ou "Vertical".</summary>
    public static readonly DependencyProperty ScrollHostProperty =
        DependencyProperty.RegisterAttached("ScrollHost", typeof(string), typeof(Nav), new PropertyMetadata(null));

    /// <summary>Elemento visível que limita a navegação (ex.: painel aberto); enquanto visível, só ele é navegável.</summary>
    public static readonly DependencyProperty IsModalScopeProperty =
        DependencyProperty.RegisterAttached("IsModalScope", typeof(bool), typeof(Nav), new PropertyMetadata(false));

    /// <summary>← → não saem do grupo (ex.: fileiras da Central de Controle).</summary>
    public static readonly DependencyProperty LockHorizontalProperty =
        DependencyProperty.RegisterAttached("LockHorizontal", typeof(bool), typeof(Nav), new PropertyMetadata(false));

    /// <summary>Liga elementos de grupos diferentes na vertical (ex.: cartão "audio" ↔ ícone "audio").</summary>
    public static readonly DependencyProperty LinkProperty =
        DependencyProperty.RegisterAttached("Link", typeof(string), typeof(Nav), new PropertyMetadata(null));

    /// <summary>Ponto de entrada do grupo: ao entrar no grupo vindo de fora, o foco vai para cá (ex.: item selecionado da barra lateral).</summary>
    public static readonly DependencyProperty IsGroupEntryProperty =
        DependencyProperty.RegisterAttached("IsGroupEntry", typeof(bool), typeof(Nav), new PropertyMetadata(false));

    public static bool GetIsGroupEntry(DependencyObject d) => (bool)d.GetValue(IsGroupEntryProperty);

    public static void SetIsGroupEntry(DependencyObject d, bool value) => d.SetValue(IsGroupEntryProperty, value);

    public static string? GetGroup(DependencyObject d) => (string?)d.GetValue(GroupProperty);

    public static void SetGroup(DependencyObject d, string? value) => d.SetValue(GroupProperty, value);

    public static string? GetKey(DependencyObject d) => (string?)d.GetValue(KeyProperty);

    public static void SetKey(DependencyObject d, string? value) => d.SetValue(KeyProperty, value);

    public static bool GetIsDefault(DependencyObject d) => (bool)d.GetValue(IsDefaultProperty);

    public static void SetIsDefault(DependencyObject d, bool value) => d.SetValue(IsDefaultProperty, value);

    public static ICommand? GetAdjustCommand(DependencyObject d) => (ICommand?)d.GetValue(AdjustCommandProperty);

    public static void SetAdjustCommand(DependencyObject d, ICommand? value) => d.SetValue(AdjustCommandProperty, value);

    public static string? GetScrollHost(DependencyObject d) => (string?)d.GetValue(ScrollHostProperty);

    public static void SetScrollHost(DependencyObject d, string? value) => d.SetValue(ScrollHostProperty, value);

    public static bool GetIsModalScope(DependencyObject d) => (bool)d.GetValue(IsModalScopeProperty);

    public static void SetIsModalScope(DependencyObject d, bool value) => d.SetValue(IsModalScopeProperty, value);

    public static bool GetLockHorizontal(DependencyObject d) => (bool)d.GetValue(LockHorizontalProperty);

    public static void SetLockHorizontal(DependencyObject d, bool value) => d.SetValue(LockHorizontalProperty, value);

    public static string? GetLink(DependencyObject d) => (string?)d.GetValue(LinkProperty);

    public static void SetLink(DependencyObject d, string? value) => d.SetValue(LinkProperty, value);
}
