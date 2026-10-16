using System.Text;

namespace Launcher.Core.Library.BattleNet;

/// <summary>Um produto instalado segundo o <c>product.db</c> do Battle.net.</summary>
public sealed record BattleNetInstall(string Uid, string? ProductCode, string InstallPath, bool? Installed);

/// <summary>
/// Leitura mínima do <c>%ProgramData%\Battle.net\Agent\product.db</c> (protobuf), sem pacote: só os campos usados.
/// <c>ProductDb.product_installs</c> (1) → <c>ProductInstall</c> { <c>uid</c> (1), <c>product_code</c> (2),
/// <c>settings</c> (3) → <c>install_path</c> (1), <c>cached_product_state</c> (4) → <c>base_product_state</c> (1) →
/// <c>installed</c> (1) }. Campos desconhecidos são pulados; arquivo truncado ou inválido lança
/// <see cref="InvalidDataException"/>.
/// </summary>
public static class ProductDb
{
    public static IReadOnlyList<BattleNetInstall> Parse(ReadOnlySpan<byte> data)
    {
        var installs = new List<BattleNetInstall>();
        var reader = new ProtoReader(data);
        while (reader.Next(out int field, out int wire))
        {
            if (field == 1 && wire == ProtoReader.LengthDelimited)
            {
                if (ParseInstall(reader.ReadBytes()) is { } install)
                {
                    installs.Add(install);
                }
            }
            else
            {
                reader.Skip(wire);
            }
        }

        return installs;
    }

    private static BattleNetInstall? ParseInstall(ReadOnlySpan<byte> data)
    {
        string? uid = null, code = null, path = null;
        bool? installed = null;
        var reader = new ProtoReader(data);
        while (reader.Next(out int field, out int wire))
        {
            switch (field, wire)
            {
                case (1, ProtoReader.LengthDelimited):
                    uid = reader.ReadString();
                    break;
                case (2, ProtoReader.LengthDelimited):
                    code = reader.ReadString();
                    break;
                case (3, ProtoReader.LengthDelimited):
                    path = FirstString(reader.ReadBytes(), 1);
                    break;
                case (4, ProtoReader.LengthDelimited):
                    installed = Installed(reader.ReadBytes());
                    break;
                default:
                    reader.Skip(wire);
                    break;
            }
        }

        return string.IsNullOrWhiteSpace(uid) || string.IsNullOrWhiteSpace(path) ? null : new BattleNetInstall(uid, code, path, installed);
    }

    /// <summary><c>cached_product_state.base_product_state.installed</c>; nulo se o arquivo não informa.</summary>
    private static bool? Installed(ReadOnlySpan<byte> cached)
    {
        var reader = new ProtoReader(cached);
        while (reader.Next(out int field, out int wire))
        {
            if (field == 1 && wire == ProtoReader.LengthDelimited)
            {
                var state = new ProtoReader(reader.ReadBytes());
                while (state.Next(out int stateField, out int stateWire))
                {
                    if (stateField == 1 && stateWire == ProtoReader.Varint)
                    {
                        return state.ReadVarint() != 0;
                    }

                    state.Skip(stateWire);
                }

                return null;
            }

            reader.Skip(wire);
        }

        return null;
    }

    private static string? FirstString(ReadOnlySpan<byte> message, int wanted)
    {
        var reader = new ProtoReader(message);
        while (reader.Next(out int field, out int wire))
        {
            if (field == wanted && wire == ProtoReader.LengthDelimited)
            {
                return reader.ReadString();
            }

            reader.Skip(wire);
        }

        return null;
    }

    /// <summary>Leitor do formato de fio do protobuf (varint, 64 bits, tamanho+bytes, 32 bits).</summary>
    private ref struct ProtoReader(ReadOnlySpan<byte> data)
    {
        public const int Varint = 0;
        public const int Fixed64 = 1;
        public const int LengthDelimited = 2;
        public const int Fixed32 = 5;

        private readonly ReadOnlySpan<byte> _data = data;
        private int _position;

        public bool Next(out int field, out int wire)
        {
            if (_position >= _data.Length)
            {
                field = wire = 0;
                return false;
            }

            ulong tag = ReadVarint();
            field = (int)(tag >> 3);
            wire = (int)(tag & 7);
            if (field <= 0)
            {
                throw new InvalidDataException("product.db: campo inválido");
            }

            return true;
        }

        public ulong ReadVarint()
        {
            ulong value = 0;
            for (int shift = 0; shift < 64; shift += 7)
            {
                if (_position >= _data.Length)
                {
                    throw new InvalidDataException("product.db: arquivo truncado");
                }

                byte b = _data[_position++];
                value |= (ulong)(b & 0x7F) << shift;
                if ((b & 0x80) == 0)
                {
                    return value;
                }
            }

            throw new InvalidDataException("product.db: número inválido");
        }

        public ReadOnlySpan<byte> ReadBytes()
        {
            ulong length = ReadVarint();
            if (length > (ulong)(_data.Length - _position))
            {
                throw new InvalidDataException("product.db: arquivo truncado");
            }

            var slice = _data.Slice(_position, (int)length);
            _position += (int)length;
            return slice;
        }

        public string ReadString() => Encoding.UTF8.GetString(ReadBytes());

        public void Skip(int wire)
        {
            switch (wire)
            {
                case Varint:
                    ReadVarint();
                    break;
                case Fixed64:
                    Advance(8);
                    break;
                case LengthDelimited:
                    ReadBytes();
                    break;
                case Fixed32:
                    Advance(4);
                    break;
                default:
                    throw new InvalidDataException("product.db: tipo de campo não suportado");
            }
        }

        private void Advance(int count)
        {
            if (count > _data.Length - _position)
            {
                throw new InvalidDataException("product.db: arquivo truncado");
            }

            _position += count;
        }
    }
}
