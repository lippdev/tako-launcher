using System.Windows.Markup;

// Temas Playnite usam controles e extensões sem prefixo: o Playnite os registra no namespace do WPF.
// O adaptador faz o mesmo, então o XAML do tema resolve para as implementações daqui sem reescrita.
[assembly: XmlnsDefinition("http://schemas.microsoft.com/winfx/2006/xaml/presentation", "Launcher.PlayniteCompat.P10.Controls")]
[assembly: XmlnsDefinition("http://schemas.microsoft.com/winfx/2006/xaml/presentation", "Launcher.PlayniteCompat.P10.Markup")]
