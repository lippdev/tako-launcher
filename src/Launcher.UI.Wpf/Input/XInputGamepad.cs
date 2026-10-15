using System.Globalization;
using System.Runtime.InteropServices;
using Launcher.Core.Input;

namespace Launcher.UI.Wpf.Input;

/// <summary>
/// Leitura de controles XInput (Xbox e compatíveis, inclusive os virtuais da Steam e do DS4Windows). O botão Guia só
/// aparece em <c>XInputGetStateEx</c> (ordinal 100, não documentado); sem ele, usamos <c>XInputGetState</c> e o Guia não
/// é lido (o atalho padrão da Central, View + Y, não depende dele). Controles PlayStation são lidos por HID em
/// <see cref="SonyHidGamepad"/>.
/// </summary>
internal static partial class XInputGamepad
{
    private const int MaxControllers = 4;
    private const uint ErrorSuccess = 0;
    private const ushort GuideBit = 0x0400;
    private const long EmptySlotProbeMilliseconds = 1000;
    private static readonly long[] NextEmptyProbe = new long[MaxControllers];
    private static bool _guideApiAvailable = true;

    /// <summary>Uma posição XInput lida agora: o que a navegação usa e os valores crus para o teste de controle.</summary>
    public readonly record struct Reading(int Slot, GamepadSnapshot Snapshot, ushort RawButtons, byte LeftTrigger, byte RightTrigger, short RightX, short RightY);

    /// <summary>
    /// Todos os controles XInput conectados (a navegação junta todos, <see cref="GamepadFusion"/>). Posição vazia só é
    /// consultada de novo a cada 1 s: <c>XInputGetState</c> numa posição sem controle é lento.
    /// </summary>
    public static IReadOnlyList<Reading> ReadAll()
    {
        var list = new List<Reading>();
        long now = Environment.TickCount64;
        for (int i = 0; i < MaxControllers; i++)
        {
            if (now < NextEmptyProbe[i])
            {
                continue;
            }

            if (TryRead(i, out var reading))
            {
                list.Add(reading);
            }
            else
            {
                NextEmptyProbe[i] = now + EmptySlotProbeMilliseconds;
            }
        }

        return list;
    }

    /// <summary>Posições (0–3) com controle XInput conectado agora, para o diagnóstico do sistema (MVP-034).</summary>
    public static IReadOnlyList<int> ConnectedSlots()
    {
        var slots = new List<int>();
        for (int i = 0; i < MaxControllers; i++)
        {
            if (TryRead(i, out _))
            {
                slots.Add(i);
            }
        }

        return slots;
    }

    /// <summary>Estado de cada XInput conectado para o teste de controle (botões crus em hexadecimal).</summary>
    public static IReadOnlyList<ControllerProbe> Probes()
    {
        var probes = new List<ControllerProbe>();
        for (int i = 0; i < MaxControllers; i++)
        {
            if (TryRead(i, out var r))
            {
                string state = ControllerTest.Describe(r.Snapshot)
                    + string.Create(CultureInfo.InvariantCulture, $" · analógico dir.: x {r.RightX / 32767d:+0.00;-0.00;+0.00} y {r.RightY / 32767d:+0.00;-0.00;+0.00} · LT {r.LeftTrigger} RT {r.RightTrigger}");
                string raw = string.Create(CultureInfo.InvariantCulture, $"agora: botões 0x{r.RawButtons:X4}");
                probes.Add(new ControllerProbe("XInput", $"posição {i + 1} (Xbox ou virtual da Steam/DS4Windows)", null, null, null, null, state, [raw]));
            }
        }

        return probes;
    }

    private static bool TryRead(int index, out Reading reading)
    {
        reading = default;
        XInputState state;
        uint result;
        if (_guideApiAvailable)
        {
            try
            {
                result = XInputGetStateEx((uint)index, out state);
            }
            catch (EntryPointNotFoundException)
            {
                _guideApiAvailable = false;
                result = XInputGetState((uint)index, out state);
            }
        }
        else
        {
            result = XInputGetState((uint)index, out state);
        }

        if (result != ErrorSuccess)
        {
            return false;
        }

        var pad = state.Gamepad;
        var buttons = (GamepadButtons)(pad.Buttons & ~GuideBit);
        if ((pad.Buttons & GuideBit) != 0)
        {
            buttons |= GamepadButtons.Guide;
        }

        var snapshot = new GamepadSnapshot(buttons, pad.ThumbLX / 32767d, pad.ThumbLY / 32767d);
        reading = new Reading(index, snapshot, pad.Buttons, pad.LeftTrigger, pad.RightTrigger, pad.ThumbRX, pad.ThumbRY);
        return true;
    }

    [LibraryImport("xinput1_4.dll", EntryPoint = "XInputGetState")]
    private static partial uint XInputGetState(uint userIndex, out XInputState state);

    [LibraryImport("xinput1_4.dll", EntryPoint = "#100")]
    private static partial uint XInputGetStateEx(uint userIndex, out XInputState state);

    [StructLayout(LayoutKind.Sequential)]
    private struct XInputState
    {
        public uint PacketNumber;
        public XInputGamepadState Gamepad;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct XInputGamepadState
    {
        public ushort Buttons;
        public byte LeftTrigger;
        public byte RightTrigger;
        public short ThumbLX;
        public short ThumbLY;
        public short ThumbRX;
        public short ThumbRY;
    }
}
