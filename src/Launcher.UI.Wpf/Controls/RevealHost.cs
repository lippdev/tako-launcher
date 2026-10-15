using System.Windows.Controls;
using Launcher.UI.Wpf.Navigation;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Liga uma página com partes escondidas (<see cref="IKeyRevealer"/>, no DataContext) à navegação: quando o
/// <see cref="FocusNavigator"/> procura uma chave que não está na tela, a página mostra a parte dela.
/// </summary>
public sealed class RevealHost : Decorator, IVirtualNavHost
{
    public bool RealizeKey(string key) => DataContext is IKeyRevealer revealer && revealer.RevealKey(key);
}
