namespace Launcher.Core.Input;

/// <summary>
/// Junta as leituras de todas as fontes de controle (XInput e PlayStation por HID) num estado só, antes do
/// <see cref="GamepadInterpreter"/>. Botões são a união: um "solto" de uma fonte nunca cancela o "pressionado" de outra
/// (ex.: um XInput virtual em repouso da Steam ou do DS4Windows não apaga o ↑ lido por HID). Como o interpretador
/// dispara na borda do estado unido, o mesmo controle visto pelas duas fontes aciona o comando uma vez só.
/// Em cada eixo vale a fonte mais inclinada, para um analógico em repouso (ou com leve desvio) não anular o outro.
/// </summary>
public static class GamepadFusion
{
    public static GamepadSnapshot? Merge(IEnumerable<GamepadSnapshot?> sources)
    {
        ArgumentNullException.ThrowIfNull(sources);
        GamepadSnapshot? merged = null;
        foreach (var source in sources)
        {
            if (source is not { } pad)
            {
                continue;
            }

            merged = merged is { } m
                ? new GamepadSnapshot(m.Buttons | pad.Buttons, Stronger(m.LeftX, pad.LeftX), Stronger(m.LeftY, pad.LeftY))
                : pad;
        }

        return merged;
    }

    public static GamepadSnapshot? Merge(params GamepadSnapshot?[] sources) => Merge((IEnumerable<GamepadSnapshot?>)sources);

    private static double Stronger(double a, double b) => Math.Abs(b) > Math.Abs(a) ? b : a;
}
