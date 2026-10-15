using System.Globalization;
using System.Text;

namespace Launcher.Core.Input;

/// <summary>
/// Um controle visto agora por uma fonte de entrada, para o teste de controle e o diagnóstico do sistema. Sem caminho
/// de dispositivo nem número de série (o caminho Bluetooth traz o endereço do controle): só ids USB, ligação e estado.
/// </summary>
/// <param name="Source">"HID" (PlayStation lido direto) ou "XInput" (Xbox, ou controle virtual da Steam/DS4Windows).</param>
/// <param name="Model">Modelo ou posição (ex.: "DualSense Edge", "posição 1").</param>
/// <param name="Connection">USB ou Bluetooth, quando a fonte sabe.</param>
/// <param name="Layout">Formato do último relatório reconhecido (ex.: "0x31 Bluetooth completo, 78 bytes").</param>
/// <param name="State">Botões, direcional e analógicos agora (<see cref="ControllerTest.Describe(GamepadSnapshot)"/>).</param>
/// <param name="RawReports">Últimos relatórios HID crus em hexadecimal, do mais novo para o mais velho.</param>
public sealed record ControllerProbe(
    string Source,
    string Model,
    HidTransport? Connection,
    ushort? VendorId,
    ushort? ProductId,
    string? Layout,
    string State,
    IReadOnlyList<string> RawReports);

/// <summary>Textos do teste de controle (Configurações › Diagnóstico) e da seção do relatório de diagnóstico.</summary>
public static class ControllerTest
{
    /// <summary>Bytes mostrados de cada relatório cru: botões, gatilhos e os extras do Edge cabem nos primeiros 12.</summary>
    public const int HexBytes = 32;

    private static readonly CultureInfo C = CultureInfo.InvariantCulture;

    private static readonly (GamepadButtons Button, string Label)[] Buttons =
    [
        (GamepadButtons.A, "A/✕"),
        (GamepadButtons.B, "B/○"),
        (GamepadButtons.X, "X/□"),
        (GamepadButtons.Y, "Y/△"),
        (GamepadButtons.LeftShoulder, "LB/L1"),
        (GamepadButtons.RightShoulder, "RB/R1"),
        (GamepadButtons.View, "View/Create"),
        (GamepadButtons.Start, "Menu/Options"),
        (GamepadButtons.Guide, "Guia/PS"),
    ];

    private static readonly (SonyExtraButtons Button, string Label)[] Extras =
    [
        (SonyExtraButtons.L2, "L2"),
        (SonyExtraButtons.R2, "R2"),
        (SonyExtraButtons.L3, "L3"),
        (SonyExtraButtons.R3, "R3"),
        (SonyExtraButtons.Touchpad, "touchpad"),
        (SonyExtraButtons.Mute, "mudo"),
        (SonyExtraButtons.LeftFunction, "Fn esq."),
        (SonyExtraButtons.RightFunction, "Fn dir."),
        (SonyExtraButtons.LeftPaddle, "traseiro esq."),
        (SonyExtraButtons.RightPaddle, "traseiro dir."),
    ];

    /// <summary>Direcional, botões e analógico esquerdo (o que a navegação usa).</summary>
    public static string Describe(GamepadSnapshot pad)
    {
        var dpad = new List<string>();
        if (pad.Buttons.HasFlag(GamepadButtons.DPadUp))
        {
            dpad.Add("cima");
        }

        if (pad.Buttons.HasFlag(GamepadButtons.DPadDown))
        {
            dpad.Add("baixo");
        }

        if (pad.Buttons.HasFlag(GamepadButtons.DPadLeft))
        {
            dpad.Add("esquerda");
        }

        if (pad.Buttons.HasFlag(GamepadButtons.DPadRight))
        {
            dpad.Add("direita");
        }

        var buttons = Buttons.Where(b => pad.Buttons.HasFlag(b.Button)).Select(b => b.Label).ToList();
        return $"direcional: {(dpad.Count == 0 ? "solto" : string.Join('+', dpad))} · botões: {(buttons.Count == 0 ? "nenhum" : string.Join(' ', buttons))} · analógico esq.: {Axes(pad.LeftX, pad.LeftY)}";
    }

    /// <summary>Tudo o que o relatório PlayStation traz: chapéu cru, analógico direito, gatilhos e botões extras.</summary>
    public static string Describe(SonyHidReading reading)
    {
        var extras = Extras.Where(e => reading.Extra.HasFlag(e.Button)).Select(e => e.Label).ToList();
        return Describe(reading.Snapshot)
            + string.Create(C, $" · chapéu {reading.Hat} · analógico dir.: {Axes(reading.RightX, reading.RightY)} · L2 {reading.LeftTrigger} R2 {reading.RightTrigger}")
            + (extras.Count == 0 ? string.Empty : " · extras: " + string.Join(' ', extras));
    }

    /// <summary>Formato do relatório para o teste: id, layout reconhecido e tamanho entregue pelo Windows.</summary>
    public static string LayoutLabel(SonyReportLayout? layout, byte reportId, int length)
    {
        string name = layout switch
        {
            SonyReportLayout.DualSenseUsb => "USB completo",
            SonyReportLayout.DualSenseBluetooth => "Bluetooth completo",
            SonyReportLayout.Simple => "simples",
            SonyReportLayout.DualShock4Bluetooth => "Bluetooth completo (DualShock 4)",
            _ => "não reconhecido",
        };
        return string.Create(C, $"0x{reportId:X2} {name}, {length} bytes");
    }

    /// <summary>Bytes em hexadecimal separados por espaço, cortados em <paramref name="max"/> (com "…").</summary>
    public static string Hex(ReadOnlySpan<byte> bytes, int max = HexBytes)
    {
        var text = new StringBuilder(Math.Min(bytes.Length, max) * 3);
        for (int i = 0; i < bytes.Length && i < max; i++)
        {
            if (i > 0)
            {
                text.Append(' ');
            }

            text.Append(bytes[i].ToString("X2", C));
        }

        return bytes.Length > max ? text.Append(" …").ToString() : text.ToString();
    }

    /// <summary>Uma linha de título por controle, o estado e os relatórios crus (seção do diagnóstico e da tela de teste).</summary>
    public static IReadOnlyList<string> Lines(IReadOnlyList<ControllerProbe> probes, bool? steamRunning)
    {
        ArgumentNullException.ThrowIfNull(probes);
        var lines = new List<string>
        {
            "Steam em execução: " + steamRunning switch { true => "sim", false => "não", null => "?" },
        };
        if (probes.Count == 0)
        {
            lines.Add("nenhum controle lido agora");
        }

        foreach (var probe in probes)
        {
            var header = new StringBuilder($"{probe.Source} · {probe.Model}");
            if (probe.Connection is { } connection)
            {
                header.Append(connection == HidTransport.Usb ? " · USB" : " · Bluetooth");
            }

            if (probe.VendorId is { } vendor && probe.ProductId is { } product)
            {
                header.Append(C, $" · {vendor:X4}:{product:X4}");
            }

            if (probe.Layout is { Length: > 0 } layout)
            {
                header.Append(" · relatório ").Append(layout);
            }

            lines.Add(header.ToString());
            lines.Add("  " + probe.State);
            foreach (string raw in probe.RawReports)
            {
                lines.Add("  " + raw);
            }
        }

        return lines;
    }

    private static string Axes(double x, double y) => string.Create(C, $"x {x:+0.00;-0.00;+0.00} y {y:+0.00;-0.00;+0.00}");
}
