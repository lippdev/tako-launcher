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
    /// Resultado de "Procurar jogos instalados": "12 jogos da Steam e 3 do Xbox encontrados". <paramref name="xbox"/>
    /// nulo = fonte Xbox desligada.
    /// </summary>
    public static string ScanSummary(int steam, int? xbox)
    {
        int fromXbox = xbox ?? 0;
        if (steam + fromXbox == 0)
        {
            return xbox is null ? "Nenhum jogo da Steam encontrado neste PC" : "Nenhum jogo da Steam ou do Xbox encontrado neste PC";
        }

        string found = steam + fromXbox == 1 ? "encontrado" : "encontrados";
        return (steam, fromXbox) switch
        {
            (_, 0) => $"{GameCount(steam)} da Steam {found}",
            (0, _) => $"{GameCount(fromXbox)} do Xbox {found}",
            _ => $"{GameCount(steam)} da Steam e {fromXbox} do Xbox {found}",
        };
    }
}
