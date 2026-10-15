// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/IconPixels.cs
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Ícone de programa em BGRA pré-multiplicado, linha de cima primeiro.</summary>
public sealed record IconBitmap(int Width, int Height, byte[] Pixels);

/// <summary>
/// Regras de pixel dos ícones do alternador de janelas. O Windows entrega o ícone em BGRA com alfa direto ou, nos
/// ícones antigos, sem alfa e com uma máscara à parte; a tela precisa de BGRA pré-multiplicado.
/// </summary>
public static class IconPixels
{
    /// <summary>
    /// Converte BGRA com alfa direto em pré-multiplicado, no próprio vetor. Ícone sem nenhum alfa usa a
    /// <paramref name="mask"/> (BGRA do mesmo tamanho: pixel não preto é transparente) ou fica opaco sem ela.
    /// </summary>
    public static void Premultiply(byte[] color, byte[]? mask)
    {
        ArgumentNullException.ThrowIfNull(color);
        bool hasAlpha = false;
        for (int i = 3; i < color.Length; i += 4)
        {
            if (color[i] != 0)
            {
                hasAlpha = true;
                break;
            }
        }

        for (int i = 0; i + 3 < color.Length; i += 4)
        {
            byte alpha = hasAlpha ? color[i + 3]
                : mask is not null && i + 2 < mask.Length && (mask[i] | mask[i + 1] | mask[i + 2]) != 0 ? (byte)0
                : (byte)255;
            color[i] = (byte)(color[i] * alpha / 255);
            color[i + 1] = (byte)(color[i + 1] * alpha / 255);
            color[i + 2] = (byte)(color[i + 2] * alpha / 255);
            color[i + 3] = alpha;
        }
    }

    /// <summary>Nada para ver: todo pixel é totalmente transparente (ou não há pixels).</summary>
    public static bool IsBlank(byte[] pixels)
    {
        ArgumentNullException.ThrowIfNull(pixels);
        for (int i = 3; i < pixels.Length; i += 4)
        {
            if (pixels[i] != 0)
            {
                return false;
            }
        }

        return true;
    }
}
