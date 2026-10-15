// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/Tv/WakeOnLan.cs
using System.Globalization;
using System.Net;
using System.Net.Sockets;

namespace ConsoleMode.Runtime.Tv;

/// <summary>"Pacote mágico" do Wake-on-LAN: 6 × 0xFF seguidos do endereço MAC 16 vezes.</summary>
public static class WakeOnLan
{
    /// <summary>Aceita AA:BB:CC:DD:EE:FF, AA-BB-CC-DD-EE-FF, AABB.CCDD.EEFF ou AABBCCDDEEFF.</summary>
    public static bool TryParseMac(string? text, out byte[] mac)
    {
        mac = [];
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        string hex = text.Trim().Replace(":", string.Empty, StringComparison.Ordinal)
            .Replace("-", string.Empty, StringComparison.Ordinal)
            .Replace(".", string.Empty, StringComparison.Ordinal);
        if (hex.Length != 12)
        {
            return false;
        }

        var bytes = new byte[6];
        for (int i = 0; i < 6; i++)
        {
            if (!byte.TryParse(hex.AsSpan(i * 2, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out bytes[i]))
            {
                return false;
            }
        }

        mac = bytes;
        return true;
    }

    public static byte[] BuildPacket(byte[] mac)
    {
        ArgumentNullException.ThrowIfNull(mac);
        if (mac.Length != 6)
        {
            throw new ArgumentException("O MAC tem 6 bytes", nameof(mac));
        }

        var packet = new byte[6 + (16 * 6)];
        Array.Fill(packet, (byte)0xFF, 0, 6);
        for (int i = 0; i < 16; i++)
        {
            mac.CopyTo(packet, 6 + (i * 6));
        }

        return packet;
    }

    /// <summary>Envia em broadcast nas portas usuais (9 e 7), algumas vezes para o caso de um pacote se perder.</summary>
    public static async Task SendAsync(byte[] mac, CancellationToken cancellationToken)
    {
        byte[] packet = BuildPacket(mac);
        using var udp = new UdpClient { EnableBroadcast = true };
        for (int attempt = 0; attempt < 3; attempt++)
        {
            foreach (int port in (int[])[9, 7])
            {
                await udp.SendAsync(packet, new IPEndPoint(IPAddress.Broadcast, port), cancellationToken).ConfigureAwait(false);
            }

            await Task.Delay(100, cancellationToken).ConfigureAwait(false);
        }
    }
}
