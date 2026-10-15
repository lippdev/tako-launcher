using System.Runtime.InteropServices;
using Launcher.Core.Input;

namespace Launcher.UI.Wpf.Input;

/// <summary>
/// Leitura de controles XInput (Xbox e compatíveis). O botão Guia só aparece em <c>XInputGetStateEx</c>
/// (ordinal 100, não documentado); sem ele, usamos <c>XInputGetState</c> e o botão View faz o papel do Guia.
/// Controles PlayStation precisam de Steam Input/DS4Windows até a camada GameInput (MVP-031).
/// </summary>
internal static partial class XInputGamepad
{
    private const int MaxControllers = 4;
    private const uint ErrorSuccess = 0;
    private const ushort GuideBit = 0x0400;
    private static bool _guideApiAvailable = true;

    /// <summary>Primeiro controle conectado, ou nulo.</summary>
    public static (int Index, GamepadSnapshot Snapshot)? ReadFirst()
    {
        for (int i = 0; i < MaxControllers; i++)
        {
            if (TryRead(i, out var snapshot))
            {
                return (i, snapshot);
            }
        }

        return null;
    }

    private static bool TryRead(int index, out GamepadSnapshot snapshot)
    {
        snapshot = default;
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

        snapshot = new GamepadSnapshot(buttons, pad.ThumbLX / 32767d, pad.ThumbLY / 32767d);
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
