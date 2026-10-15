using System.Globalization;
using Launcher.Core.Library;

namespace Launcher.Core.Presentation;

/// <summary>Textos derivados do jogo, em pt-BR (localização completa fica para depois do MVP).</summary>
public static class GameText
{
    private static readonly CultureInfo PtBr = CultureInfo.GetCultureInfo("pt-BR");

    public static string StoreName(GameStore store) => store switch
    {
        GameStore.Steam => "Steam",
        GameStore.Epic => "Epic Games",
        GameStore.Gog => "GOG",
        GameStore.Xbox => "Xbox",
        _ => "Adicionado",
    };

    /// <summary>"da Steam", "da Epic Games", "do GOG", "do Xbox" (para frases como "jogos da Steam").</summary>
    public static string StoreOf(GameStore store) => (store is GameStore.Gog or GameStore.Xbox ? "do " : "da ") + StoreName(store);

    /// <summary>"Agora mesmo", "Jogado ontem", "Há 3 dias", "Há 2 semanas"...</summary>
    public static string LastPlayed(DateTimeOffset? lastPlayed, DateTimeOffset now)
    {
        if (lastPlayed is not DateTimeOffset at)
        {
            return "Nunca jogado";
        }

        int days = (int)Math.Floor((now - at).TotalDays);
        return days switch
        {
            < 1 => "Agora mesmo",
            1 => "Jogado ontem",
            < 7 => $"Há {days} dias",
            < 14 => "Há 1 semana",
            < 30 => $"Há {days / 7} semanas",
            < 60 => "Há 1 mês",
            _ => $"Há {days / 30} meses",
        };
    }

    public static string PlayTime(TimeSpan playTime) =>
        playTime < TimeSpan.FromHours(1) ? (playTime == TimeSpan.Zero ? "—" : $"{Math.Max(1, (int)playTime.TotalMinutes)} min") : $"{(int)playTime.TotalHours} h";

    /// <summary>Duração de uma sessão no aviso de retorno: "menos de 1 min jogado", "25 min jogados", "1 h 05 min jogados".</summary>
    public static string SessionPlayed(TimeSpan played)
    {
        int minutes = (int)played.TotalMinutes;
        return minutes switch
        {
            < 1 => "menos de 1 min jogado",
            1 => "1 min jogado",
            < 60 => $"{minutes} min jogados",
            _ when minutes % 60 == 0 => $"{minutes / 60} h jogadas",
            _ => $"{minutes / 60} h {(minutes % 60).ToString("00", PtBr)} min jogados",
        };
    }

    public static string Date(DateOnly date) => date.ToString("dd/MM/yyyy", PtBr);

    public static string ShortDate(DateOnly date) => date.ToString("dd/MM", PtBr);

    public static string Genres(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return string.Join(" · ", game.Genres);
    }

    public static string Status(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return game.UpcomingRelease is DateOnly up ? $"Lança em {Date(up)}" : game.IsInstalled ? "Instalado" : "Não instalado";
    }
}
