// Adaptado de lippdev/consolemode: tests/ConsoleMode.Tests/TvControlTests.cs
using System.Buffers.Binary;
using System.Net.Sockets;
using System.Numerics;
using System.Security.Cryptography;
using System.Text;
using ConsoleMode.Runtime.Tv;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public class TvControlTests
{
    [Fact]
    public void Adb_messages_carry_checksum_and_magic_and_decode_back()
    {
        byte[] banner = AdbProtocol.ConnectBanner();
        byte[] bytes = AdbProtocol.Encode(AdbProtocol.Connect, AdbProtocol.Version, AdbProtocol.MaxPayload, banner);

        Assert.Equal(AdbProtocol.HeaderSize + banner.Length, bytes.Length);
        Assert.Equal("CNXN", Encoding.ASCII.GetString(bytes, 0, 4));
        Assert.Equal(AdbProtocol.Checksum(banner), BinaryPrimitives.ReadUInt32LittleEndian(bytes.AsSpan(16)));
        Assert.Equal(AdbProtocol.Connect ^ 0xFFFFFFFF, BinaryPrimitives.ReadUInt32LittleEndian(bytes.AsSpan(20)));

        var (command, arg0, arg1, length, checksum) = AdbProtocol.DecodeHeader(bytes);
        Assert.Equal(AdbProtocol.Connect, command);
        Assert.Equal(AdbProtocol.Version, arg0);
        Assert.Equal(AdbProtocol.MaxPayload, arg1);
        Assert.Equal(banner.Length, length);
        Assert.Equal(AdbProtocol.Checksum(banner), checksum);
        Assert.Equal("host::\0", Encoding.ASCII.GetString(bytes, AdbProtocol.HeaderSize, length));
    }

    [Fact]
    public void Adb_rejects_wrong_checksum_wrong_magic_and_oversized_messages()
    {
        Assert.Throws<InvalidDataException>(() => AdbProtocol.ValidatePayload(0, Encoding.ASCII.GetBytes("resposta")));

        byte[] bytes = AdbProtocol.Encode(AdbProtocol.Okay, 1, 2);
        bytes[20] ^= 0xFF;
        Assert.Throws<InvalidDataException>(() => AdbProtocol.DecodeHeader(bytes));

        byte[] big = AdbProtocol.Encode(AdbProtocol.Write, 1, 2);
        BinaryPrimitives.WriteUInt32LittleEndian(big.AsSpan(12), AdbProtocol.MaxPayload + 1);
        Assert.Throws<InvalidDataException>(() => AdbProtocol.DecodeHeader(big));
    }

    [Fact]
    public void Adb_shell_output_is_bounded_across_packets_and_keeps_text()
    {
        var output = new AdbShellOutput();
        output.Append(Encoding.UTF8.GetBytes("TV pronta"));
        Assert.Equal("TV pronta", output.ToString());

        var bounded = new AdbShellOutput();
        var packet = new byte[AdbProtocol.MaxPayload];
        for (int i = 0; i < AdbProtocol.MaxShellOutput / packet.Length; i++)
        {
            bounded.Append(packet);
        }

        Assert.Throws<InvalidDataException>(() => bounded.Append([1]));
    }

    [Fact]
    public void Adb_token_signature_verifies_as_a_sha1_digest()
    {
        using var key = RSA.Create(2048);
        byte[] token = RandomNumberGenerator.GetBytes(20);
        byte[] signature = AdbProtocol.SignToken(key, token);
        Assert.True(key.VerifyHash(token, signature, HashAlgorithmName.SHA1, RSASignaturePadding.Pkcs1));
    }

    [Fact]
    public void Adb_public_key_uses_the_android_layout()
    {
        using var key = RSA.Create(2048);
        var parameters = key.ExportParameters(false);
        string encoded = Encoding.ASCII.GetString(AdbProtocol.EncodePublicKey(parameters, "Tako@PC"));

        Assert.EndsWith(" Tako@PC\0", encoded, StringComparison.Ordinal);
        byte[] raw = Convert.FromBase64String(encoded[..encoded.IndexOf(' ', StringComparison.Ordinal)]);
        Assert.Equal(524, raw.Length);
        Assert.Equal(64u, BinaryPrimitives.ReadUInt32LittleEndian(raw));

        var modulus = new BigInteger(raw.AsSpan(8, 256), isUnsigned: true, isBigEndian: false);
        Assert.Equal(new BigInteger(parameters.Modulus, isUnsigned: true, isBigEndian: true), modulus);

        // n0inv = -1 / n mod 2^32
        uint n0 = (uint)(modulus & uint.MaxValue);
        uint n0Inverse = BinaryPrimitives.ReadUInt32LittleEndian(raw.AsSpan(4));
        Assert.Equal(uint.MaxValue, unchecked(n0 * n0Inverse));

        var rr = new BigInteger(raw.AsSpan(264, 256), isUnsigned: true, isBigEndian: false);
        Assert.Equal(BigInteger.ModPow(2, 4096, modulus), rr);
        Assert.Equal(65537u, BinaryPrimitives.ReadUInt32LittleEndian(raw.AsSpan(520)));
    }

    [Theory]
    [InlineData("192.168.0.50", "192.168.0.50", 5555)]
    [InlineData(" 192.168.0.50:5556 ", "192.168.0.50", 5556)]
    [InlineData("tv.lan", "tv.lan", 5555)]
    [InlineData("192.168.0.50:abc", "192.168.0.50:abc", 5555)]
    [InlineData("192.168.0.50:70000", "192.168.0.50:70000", 5555)]
    public void Adb_endpoint_defaults_to_port_5555(string input, string host, int port) =>
        Assert.Equal((host, port), AdbProtocol.ParseEndpoint(input));

    [Theory]
    [InlineData(1, 243)]
    [InlineData(4, 246)]
    [InlineData(0, 243)]
    [InlineData(9, 246)]
    public void Hdmi_inputs_map_to_the_android_tv_input_keys(int input, int key) =>
        Assert.Equal(key, AdbProtocol.HdmiKey(input));

    [Theory]
    [InlineData("AA:BB:CC:DD:EE:FF")]
    [InlineData("aa-bb-cc-dd-ee-ff")]
    [InlineData("AABB.CCDD.EEFF")]
    [InlineData("AABBCCDDEEFF")]
    public void Mac_addresses_parse_in_the_usual_formats(string text)
    {
        Assert.True(WakeOnLan.TryParseMac(text, out byte[] mac));
        Assert.Equal(new byte[] { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF }, mac);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("AA:BB:CC:DD:EE")]
    [InlineData("GG:BB:CC:DD:EE:FF")]
    public void Invalid_mac_addresses_are_rejected(string? text) => Assert.False(WakeOnLan.TryParseMac(text, out _));

    [Fact]
    public void Magic_packet_is_six_ff_then_the_mac_sixteen_times()
    {
        byte[] mac = [1, 2, 3, 4, 5, 6];
        byte[] packet = WakeOnLan.BuildPacket(mac);

        Assert.Equal(102, packet.Length);
        Assert.All(packet[..6], b => Assert.Equal(0xFF, b));
        for (int i = 0; i < 16; i++)
        {
            Assert.Equal(mac, packet[(6 + (i * 6))..(12 + (i * 6))]);
        }
    }

    [Fact]
    public void Tv_control_is_off_by_default_and_survives_bad_json()
    {
        Assert.False(TvControlConfig.Default.Enabled);
        Assert.False(TvControlConfig.Default.IsActive);
        Assert.Equal(TvControlConfig.Default, TvControlConfigJson.Parse(null));
        Assert.Equal(TvControlConfig.Default, TvControlConfigJson.Parse("{ isso não é json"));
        Assert.False(new TvControlConfig { Enabled = true }.IsActive); // sem endereço não há o que fazer
    }

    [Fact]
    public void Tv_control_round_trips_and_clamps_the_hdmi_input()
    {
        var config = new TvControlConfig { Enabled = true, Host = "192.168.0.50", MacAddress = "AA:BB:CC:DD:EE:FF", HdmiInput = 3, StandbyAfterGame = true };

        Assert.Equal(config, TvControlConfigJson.Parse(TvControlConfigJson.Serialize(config)));
        Assert.Equal(4, TvControlConfigJson.Parse("""{"enabled":true,"host":"tv","hdmiInput":9}""").HdmiInput);
        Assert.Equal(string.Empty, TvControlConfigJson.Parse("""{"host":null}""").Host);
    }

    [Fact]
    public void Input_switch_uses_the_custom_command_when_there_is_one()
    {
        Assert.Equal("input keyevent 244", new TvControlConfig { HdmiInput = 2 }.InputSwitchCommand);
        Assert.Equal("am start -a android.intent.action.VIEW", new TvControlConfig { HdmiInput = 2, InputCommand = "  am start -a android.intent.action.VIEW " }.InputSwitchCommand);
    }

    [Fact]
    public void Adb_key_is_created_once_then_reloaded_and_replaced_when_unreadable()
    {
        string folder = Path.Combine(Path.GetTempPath(), "tako-adb-" + Guid.NewGuid().ToString("N"));
        try
        {
            var protector = new PrefixProtector();
            var store = new AdbKeyStore(Path.Combine(folder, "adbkey.pem"), protector);

            var (created, firstOrigin) = store.LoadOrCreate();
            byte[] modulus = created.ExportParameters(false).Modulus!;
            created.Dispose();
            Assert.Equal(AdbKeyOrigin.Created, firstOrigin);
            Assert.StartsWith(PrefixProtector.Prefix, File.ReadAllText(store.FilePath), StringComparison.Ordinal);

            var (loaded, secondOrigin) = store.LoadOrCreate();
            using (loaded)
            {
                Assert.Equal(AdbKeyOrigin.Loaded, secondOrigin);
                Assert.Equal(modulus, loaded.ExportParameters(false).Modulus);
            }

            protector.Readable = false; // copiada de outro usuário/PC
            var (replaced, thirdOrigin) = store.LoadOrCreate();
            using (replaced)
            {
                Assert.Equal(AdbKeyOrigin.Replaced, thirdOrigin);
                Assert.NotEqual(modulus, replaced.ExportParameters(false).Modulus);
            }
        }
        finally
        {
            if (Directory.Exists(folder))
            {
                Directory.Delete(folder, recursive: true);
            }
        }
    }

    [Fact]
    public async Task Unreachable_tv_without_mac_fails_with_a_message_for_the_user()
    {
        var config = new TvControlConfig { Enabled = true, Host = "192.168.0.50" };
        int attempts = 0;

        var error = await Assert.ThrowsAsync<TvControlException>(() => TvNetwork.ConnectAsync<int>(
            config,
            wakeOnLan: true,
            TimeSpan.Zero,
            wakeOnLanSent: null,
            _ =>
            {
                attempts++;
                throw new SocketException((int)SocketError.ConnectionRefused);
            },
            TestContext.Current.CancellationToken));

        Assert.Equal(1, attempts);
        Assert.Contains("192.168.0.50", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Reachable_tv_connects_on_the_first_try() =>
        Assert.Equal(42, await TvNetwork.ConnectAsync(new TvControlConfig { Host = "tv" }, wakeOnLan: true, TimeSpan.Zero, null, _ => Task.FromResult(42), TestContext.Current.CancellationToken));

    /// <summary>Protetor de mentira: só marca o texto, e pode fingir que não consegue ler.</summary>
    private sealed class PrefixProtector : ISecretProtector
    {
        public const string Prefix = "teste:";

        public bool Readable { get; set; } = true;

        public string Protect(string secret) => Prefix + secret;

        public bool TryUnprotect(string stored, out string secret)
        {
            secret = Readable && stored.StartsWith(Prefix, StringComparison.Ordinal) ? stored[Prefix.Length..] : string.Empty;
            return Readable;
        }
    }
}
