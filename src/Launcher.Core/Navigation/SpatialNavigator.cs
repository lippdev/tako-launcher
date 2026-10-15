namespace Launcher.Core.Navigation;

public enum NavDirection
{
    Up,
    Down,
    Left,
    Right,
}

/// <summary>Retângulo em coordenadas de tela (independente de WPF).</summary>
public readonly record struct NavRect(double Left, double Top, double Width, double Height)
{
    public double Right => Left + Width;

    public double Bottom => Top + Height;

    public double CenterX => Left + (Width / 2);

    public double CenterY => Top + (Height / 2);
}

/// <summary>Candidato à navegação: item, posição e grupo (fileira, barra lateral...).</summary>
public readonly record struct NavCandidate<T>(T Item, NavRect Rect, string Group);

/// <summary>
/// Navegação espacial do controle (MVP-028/029): escolhe o vizinho mais próximo na direção pedida,
/// penalizando o desvio lateral. Mesmo algoritmo do protótipo em <c>design/prototype</c>.
/// </summary>
public static class SpatialNavigator
{
    private const double Tolerance = 2;
    private const double OrthogonalWeight = 2.2;

    public static NavCandidate<T>? FindNext<T>(NavRect from, IEnumerable<NavCandidate<T>> candidates, NavDirection direction)
    {
        ArgumentNullException.ThrowIfNull(candidates);
        NavCandidate<T>? best = null;
        double bestScore = double.MaxValue;
        foreach (var candidate in candidates)
        {
            var c = candidate.Rect;
            if (c == from)
            {
                continue;
            }

            double primary, orthogonal;
            switch (direction)
            {
                case NavDirection.Left when !(c.Right > from.Left + Tolerance && c.CenterX >= from.CenterX - Tolerance):
                    (primary, orthogonal) = (from.Left - c.Right, Math.Abs(c.CenterY - from.CenterY));
                    break;
                case NavDirection.Right when !(c.Left < from.Right - Tolerance && c.CenterX <= from.CenterX + Tolerance):
                    (primary, orthogonal) = (c.Left - from.Right, Math.Abs(c.CenterY - from.CenterY));
                    break;
                case NavDirection.Up when !(c.Bottom > from.Top + Tolerance && c.CenterY >= from.CenterY - Tolerance):
                    (primary, orthogonal) = (from.Top - c.Bottom, Math.Abs(c.CenterX - from.CenterX));
                    break;
                case NavDirection.Down when !(c.Top < from.Bottom - Tolerance && c.CenterY <= from.CenterY + Tolerance):
                    (primary, orthogonal) = (c.Top - from.Bottom, Math.Abs(c.CenterX - from.CenterX));
                    break;
                default:
                    continue;
            }

            double score = Math.Max(primary, 0) + (orthogonal * OrthogonalWeight);
            if (score < bestScore)
            {
                bestScore = score;
                best = candidate;
            }
        }

        return best;
    }
}
