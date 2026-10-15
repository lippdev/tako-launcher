using System.Windows;

namespace Launcher.PlayniteCompat.P10.Data;

#pragma warning disable CA1822 // Bindings do WPF só leem propriedades de instância; valores fixos ficam como propriedades de instância.

/// <summary>Notificações do Playnite (<c>{Api Notifications.Count}</c>). O Tako não tem notificações de extensões.</summary>
public sealed class P10Notifications
{
    public int Count => 0;
}

/// <summary>
/// Caminhos do Playnite (<c>{Api Paths.ConfigurationPath}</c>). Ficam vazios de propósito: o tema não descobre pastas
/// do usuário nem monta caminhos fora da própria pasta.
/// </summary>
public sealed class P10Paths
{
    public string? ApplicationPath => null;

    public string? ConfigurationPath => null;

    public string? ExtensionsDataPath => null;

    public bool IsPortable => false;
}

/// <summary>Informações do app (<c>{Api ApplicationInfo.Mode}</c>).</summary>
public sealed class P10ApplicationInfo
{
    public string Mode => "Fullscreen";

    public bool IsPortable => false;

    public bool InOfflineMode => true;

    public bool IsDebugBuild => false;
}

/// <summary>Visão principal (<c>{Api MainView.SelectedGames}</c>).</summary>
public sealed class P10MainView(P10MainModel? model)
{
    public IReadOnlyList<P10GameEntry> SelectedGames => model?.SelectedGame is { } game ? [game] : [];

    public P10FilterPreset? ActiveFilterPreset => model?.ActiveFilterPreset;
}

/// <summary>
/// Subconjunto só-leitura do <c>IPlayniteAPI</c> que os temas leem com <c>{Api ...}</c> (MVP-018). Não há métodos nem
/// propriedades graváveis: a extensão cria bindings OneWay, então o tema só consegue ler valores e o modelo
/// principal do adaptador. Banco de dados, plugins, diálogos, caminhos reais e recursos do Playnite ficam de fora.
/// </summary>
public sealed class P10Api(P10MainModel? model)
{
    public P10MainModel? MainModel { get; } = model;

    public P10MainView MainView { get; } = new(model);

    public P10Notifications Notifications { get; } = new();

    public P10Paths Paths { get; } = new();

    public P10ApplicationInfo ApplicationInfo { get; } = new();

    /// <summary>Atalhos para a lista de jogos e o jogo selecionado.</summary>
    public IReadOnlyCollection<P10GameEntry> Games => MainModel?.Games ?? (IReadOnlyCollection<P10GameEntry>)[];

    public P10GameEntry? SelectedGame => MainModel?.SelectedGame;
}

/// <summary>
/// Contexto do adaptador herdado pela árvore visual: a <see cref="P10Api"/> do tema em uso. A extensão <c>{Api}</c> lê
/// daqui (sem estado global), e cada tela de tema recebe a própria API.
/// </summary>
public static class P10Context
{
    public static readonly DependencyProperty ApiProperty = DependencyProperty.RegisterAttached(
        "Api", typeof(P10Api), typeof(P10Context), new FrameworkPropertyMetadata(null, FrameworkPropertyMetadataOptions.Inherits));

    public static P10Api? GetApi(DependencyObject element)
    {
        ArgumentNullException.ThrowIfNull(element);
        return (P10Api?)element.GetValue(ApiProperty);
    }

    public static void SetApi(DependencyObject element, P10Api? value)
    {
        ArgumentNullException.ThrowIfNull(element);
        element.SetValue(ApiProperty, value);
    }
}
