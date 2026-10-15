namespace ConsoleMode.Runtime.Tv;

/// <summary>Como o Tako fala com a TV pela rede.</summary>
public enum TvKind
{
    /// <summary>Google TV / Android TV (TCL, Sony, Hisense, Philips…) por ADB na porta 5555.</summary>
    Android,

    /// <summary>LG com webOS, pela API local em WebSocket (portas 3000/3001).</summary>
    LgWebOs,

    /// <summary>Samsung com Tizen (2016 em diante), pela API de controle remoto em WebSocket (portas 8001/8002).</summary>
    Samsung,
}

/// <summary>Nomes curtos para a tela e o log.</summary>
public static class TvKinds
{
    public static IReadOnlyList<TvKind> All { get; } = [TvKind.Android, TvKind.LgWebOs, TvKind.Samsung];

    public static string Label(TvKind kind) => kind switch
    {
        TvKind.LgWebOs => "LG (webOS)",
        TvKind.Samsung => "Samsung (Tizen)",
        _ => "Google TV / Android TV",
    };

    /// <summary>O próximo tipo da lista (o botão "Tipo de TV" anda em círculo).</summary>
    public static TvKind Next(TvKind kind)
    {
        int index = -1;
        for (int i = 0; i < All.Count; i++)
        {
            if (All[i] == kind)
            {
                index = i;
            }
        }

        return All[(index + 1) % All.Count];
    }
}
