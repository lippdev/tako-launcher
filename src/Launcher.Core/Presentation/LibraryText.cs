using Launcher.Core.Library;

namespace Launcher.Core.Presentation;

/// <summary>Textos da busca, dos filtros e da ordenação da biblioteca, em pt-BR.</summary>
public static class LibraryText
{
    /// <summary>Ordem das opções de ordenação no painel e no botão "Ordenar".</summary>
    public static IReadOnlyList<LibrarySort> SortOrder { get; } =
        [LibrarySort.RecentlyPlayed, LibrarySort.Name, LibrarySort.PlayTime, LibrarySort.Rating, LibrarySort.RecentlyAdded];

    public static string SortLabel(LibrarySort sort) => sort switch
    {
        LibrarySort.Name => "Nome",
        LibrarySort.PlayTime => "Tempo de jogo",
        LibrarySort.RecentlyAdded => "Data de adição",
        LibrarySort.Rating => "Nota",
        _ => "Último jogado",
    };

    public static string InstallLabel(InstallFilter install) => install switch
    {
        InstallFilter.Installed => "Instalados",
        InstallFilter.NotInstalled => "Não instalados",
        _ => "Todos",
    };

    /// <summary>A opção seguinte da lista de ordenação (volta ao início no fim).</summary>
    public static LibrarySort NextSort(LibrarySort sort)
    {
        int index = SortOrder.ToList().IndexOf(sort);
        return SortOrder[(index + 1) % SortOrder.Count];
    }

    /// <summary>"Nenhum jogo", "1 jogo", "12 jogos".</summary>
    public static string GameCount(int count) => count switch
    {
        0 => "Nenhum jogo",
        1 => "1 jogo",
        _ => $"{count} jogos",
    };

    /// <summary>"Filtros", "Filtros (2)".</summary>
    public static string FiltersLabel(int active) => active == 0 ? "Filtros" : $"Filtros ({active})";

    /// <summary>
    /// Resultado de "Procurar jogos instalados" pelas lojas ligadas (ADR-0019): "12 jogos da Steam e 3 do Xbox
    /// encontrados", "12 jogos da Steam, 2 da Epic Games e 3 do Xbox encontrados". Lojas sem jogos não aparecem.
    /// </summary>
    public static string ScanSummary(IEnumerable<(GameStore Store, int Count)> enabled)
    {
        ArgumentNullException.ThrowIfNull(enabled);
        var stores = enabled.ToList();
        var found = stores.Where(s => s.Count > 0).ToList();
        if (found.Count == 0)
        {
            return stores.Count switch
            {
                0 => "Todas as lojas estão desligadas nas Configurações",
                1 => $"Nenhum jogo {GameText.StoreOf(stores[0].Store)} encontrado neste PC",
                _ => $"Nenhum jogo {string.Join(", ", stores.SkipLast(1).Select(s => GameText.StoreOf(s.Store)))} ou {GameText.StoreOf(stores[^1].Store)} encontrado neste PC",
            };
        }

        var parts = found.Select((s, i) => $"{(i == 0 ? GameCount(s.Count) : s.Count.ToString(System.Globalization.CultureInfo.InvariantCulture))} {GameText.StoreOf(s.Store)}").ToList();
        string list = parts.Count == 1 ? parts[0] : $"{string.Join(", ", parts.SkipLast(1))} e {parts[^1]}";
        return $"{list} {(found.Sum(s => s.Count) == 1 ? "encontrado" : "encontrados")}";
    }
}
