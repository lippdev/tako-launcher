using System.Buffers.Binary;
using System.Text;

namespace Launcher.Core.Library.Playnite;

/// <summary>
/// Leitor, só de leitura, dos arquivos do LiteDB 4 (formato de arquivo "v7", páginas de 4 KB) que o Playnite usa na
/// pasta <c>library</c> (STORE-009). Lê os documentos de uma coleção pelas páginas do índice <c>_id</c> e devolve cada um
/// como dicionário (BSON → <see cref="string"/>, <see cref="bool"/>, <see cref="int"/>, <see cref="long"/>,
/// <see cref="double"/>, <see cref="Guid"/>, <see cref="DateTime"/>, listas e dicionários). Nunca escreve no arquivo.
/// Arquivo de outro formato (LiteDB 5, criptografado) ou corrompido lança <see cref="InvalidDataException"/>.
/// </summary>
// Adaptado de mbdavid/LiteDB: LiteDB/Engine/FileReader/FileReaderV7.cs (MIT), via
// JosefNemec/Playnite: source/Playnite/Database/Collections/LiteDBFileReaderV7.cs (MIT).
public sealed class LiteDbV7Reader
{
    private const int PageSize = 4096;
    private const int PageHeaderSize = 25; // pageID, tipo, anterior, próxima, itens, livres e reservado
    private const string HeaderInfo = "** This is a LiteDB file **";

    private const byte HeaderPage = 1;
    private const byte CollectionPage = 2;
    private const byte IndexPage = 3;
    private const byte DataPage = 4;
    private const byte ExtendPage = 5;

    private readonly byte[] _file;
    private readonly Dictionary<string, uint> _collections = new(StringComparer.OrdinalIgnoreCase);

    public LiteDbV7Reader(byte[] file)
    {
        ArgumentNullException.ThrowIfNull(file);
        _file = file;
        var header = Page(0) ?? throw new InvalidDataException("Arquivo vazio");
        if (header.Type != HeaderPage)
        {
            throw new InvalidDataException("Não é um arquivo do LiteDB 4");
        }

        var reader = header.Body;
        string info = reader.ReadFixedString(27);
        byte version = reader.ReadByte();
        if (info != HeaderInfo || version != 7)
        {
            throw new InvalidDataException("Não é um arquivo do LiteDB 4 (formato v7)");
        }

        reader.Skip(2 + 4 + 4); // ChangeID, FreeEmptyPageID, LastPageID
        reader.Skip(2); // userVersion
        reader.Skip(20); // senha
        if (reader.ReadBytes(16).Any(b => b != 0))
        {
            throw new InvalidDataException("Arquivo do LiteDB protegido por senha");
        }

        int count = reader.ReadByte();
        for (int i = 0; i < count; i++)
        {
            string name = reader.ReadLengthString();
            _collections[name] = reader.ReadUInt32();
        }
    }

    /// <summary>Lê o arquivo inteiro sem bloquear quem o tem aberto (o Playnite pode estar rodando).</summary>
    public static LiteDbV7Reader Open(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        using var memory = new MemoryStream();
        stream.CopyTo(memory);
        return new LiteDbV7Reader(memory.ToArray());
    }

    public IReadOnlyCollection<string> Collections => _collections.Keys;

    /// <summary>Documentos da coleção (vazio se ela não existe).</summary>
    public IReadOnlyList<Dictionary<string, object?>> Documents(string collection)
    {
        if (!_collections.TryGetValue(collection, out uint collectionPageId) || Page(collectionPageId) is not { Type: CollectionPage } page)
        {
            return [];
        }

        // Página da coleção: nome, 12 bytes e 16 índices; o primeiro é o _id, que aponta para todos os documentos.
        var reader = page.Body;
        reader.ReadLengthString();
        reader.Skip(12);
        uint? head = null;
        for (int i = 0; i < 16 && head is null; i++)
        {
            string field = reader.ReadLengthString();
            reader.Skip(1); // unique
            uint headPageId = reader.ReadUInt32();
            reader.Skip(2 + 6 + 4); // HeadNode, TailNode, FreeIndexPageID
            if (field.Length > 0)
            {
                head = headPageId;
            }
        }

        if (head is not uint start)
        {
            return [];
        }

        var documents = new List<Dictionary<string, object?>>();
        var seen = new HashSet<(uint, ushort)>();
        foreach (uint indexPageId in IndexPages(start))
        {
            foreach (var node in IndexNodes(indexPageId))
            {
                if (node.DataPage == uint.MaxValue || !seen.Add((node.DataPage, node.DataIndex)))
                {
                    continue;
                }

                byte[]? data = DataBlock(node.DataPage, node.DataIndex);
                if (data is { Length: > 0 })
                {
                    documents.Add(Bson.ReadDocument(data));
                }
            }
        }

        return documents;
    }

    private sealed record IndexNode(uint DataPage, ushort DataIndex, uint PrevPage, uint NextPage);

    private sealed record PageInfo(uint Id, byte Type, uint NextPageId, ushort ItemCount, ByteReader Body);

    private PageInfo? Page(uint id)
    {
        long offset = (long)id * PageSize;
        if (offset + PageSize > _file.Length)
        {
            return null;
        }

        var reader = new ByteReader(_file, (int)offset, PageSize);
        uint pageId = reader.ReadUInt32();
        byte type = reader.ReadByte();
        reader.ReadUInt32(); // anterior
        uint next = reader.ReadUInt32();
        ushort items = reader.ReadUInt16();
        reader.Skip(2 + 8); // bytes livres e reservado
        return new PageInfo(pageId, type, next, items, reader);
    }

    /// <summary>Todas as páginas do índice, seguindo os nós anterior/próximo a partir da primeira.</summary>
    private HashSet<uint> IndexPages(uint start)
    {
        var visited = new HashSet<uint>();
        var pending = new Stack<uint>([start]);
        while (pending.Count > 0)
        {
            uint id = pending.Pop();
            if (id == uint.MaxValue || visited.Contains(id) || Page(id) is not { Type: IndexPage })
            {
                continue;
            }

            visited.Add(id);
            foreach (var node in IndexNodes(id))
            {
                pending.Push(node.PrevPage);
                pending.Push(node.NextPage);
            }
        }

        return visited;
    }

    private List<IndexNode> IndexNodes(uint pageId)
    {
        var nodes = new List<IndexNode>();
        if (Page(pageId) is not { Type: IndexPage } page)
        {
            return nodes;
        }

        var reader = page.Body;
        for (int i = 0; i < page.ItemCount; i++)
        {
            reader.Skip(2); // índice do nó
            int levels = reader.ReadByte();
            reader.Skip(1 + 6 + 6); // slot, nó anterior e próximo
            ushort keyLength = reader.ReadUInt16();
            reader.Skip(1 + keyLength); // tipo e valor da chave
            uint dataPage = reader.ReadUInt32();
            ushort dataIndex = reader.ReadUInt16();
            uint prevPage = reader.ReadUInt32();
            reader.Skip(2);
            uint nextPage = reader.ReadUInt32();
            reader.Skip(2);
            reader.Skip(Math.Max(0, levels - 1) * (6 + 6));
            nodes.Add(new IndexNode(dataPage, dataIndex, prevPage, nextPage));
        }

        return nodes;
    }

    private byte[]? DataBlock(uint pageId, ushort index)
    {
        if (Page(pageId) is not { Type: DataPage } page)
        {
            return null;
        }

        var reader = page.Body;
        for (int i = 0; i < page.ItemCount; i++)
        {
            ushort blockIndex = reader.ReadUInt16();
            uint extendPageId = reader.ReadUInt32();
            ushort length = reader.ReadUInt16();
            byte[] data = reader.ReadBytes(length);
            if (blockIndex == index)
            {
                return extendPageId == uint.MaxValue ? data : ExtendData(extendPageId);
            }
        }

        return null;
    }

    /// <summary>Documento grande: os bytes ficam numa corrente de páginas de extensão.</summary>
    private byte[] ExtendData(uint pageId)
    {
        using var buffer = new MemoryStream();
        var visited = new HashSet<uint>();
        while (pageId != uint.MaxValue && visited.Add(pageId))
        {
            if (Page(pageId) is not { Type: ExtendPage } page)
            {
                return [];
            }

            buffer.Write(page.Body.ReadBytes(page.ItemCount));
            pageId = page.NextPageId;
        }

        return buffer.ToArray();
    }

    /// <summary>Leitura little-endian com limite: passar do fim lança <see cref="InvalidDataException"/>.</summary>
    internal sealed class ByteReader(byte[] buffer, int start, int length)
    {
        private readonly int _end = start + length;

        public int Position { get; private set; } = start;

        public void Skip(int count)
        {
            Ensure(count);
            Position += count;
        }

        public byte ReadByte()
        {
            Ensure(1);
            return buffer[Position++];
        }

        public ushort ReadUInt16() => BinaryPrimitives.ReadUInt16LittleEndian(Take(2));

        public uint ReadUInt32() => BinaryPrimitives.ReadUInt32LittleEndian(Take(4));

        public int ReadInt32() => BinaryPrimitives.ReadInt32LittleEndian(Take(4));

        public long ReadInt64() => BinaryPrimitives.ReadInt64LittleEndian(Take(8));

        public double ReadDouble() => BinaryPrimitives.ReadDoubleLittleEndian(Take(8));

        public byte[] ReadBytes(int count) => Take(count).ToArray();

        public string ReadFixedString(int count) => Encoding.UTF8.GetString(Take(count));

        /// <summary>Texto com o tamanho (int32) na frente, como o LiteDB 4 grava nomes de coleção e de índice.</summary>
        public string ReadLengthString() => ReadFixedString(ReadInt32());

        /// <summary>Texto terminado em zero (nomes de campos do BSON).</summary>
        public string ReadCString()
        {
            int zero = Array.IndexOf(buffer, (byte)0, Position, _end - Position);
            if (zero < 0)
            {
                throw new InvalidDataException("Texto sem fim no arquivo do LiteDB");
            }

            string text = Encoding.UTF8.GetString(buffer, Position, zero - Position);
            Position = zero + 1;
            return text;
        }

        private ReadOnlySpan<byte> Take(int count)
        {
            Ensure(count);
            var span = new ReadOnlySpan<byte>(buffer, Position, count);
            Position += count;
            return span;
        }

        private void Ensure(int count)
        {
            if (count < 0 || Position + count > _end)
            {
                throw new InvalidDataException("Arquivo do LiteDB truncado ou corrompido");
            }
        }
    }
}

/// <summary>BSON (o formato dos documentos do LiteDB) → dicionários e listas.</summary>
// Adaptado de mbdavid/LiteDB: LiteDB/Document/Bson/BsonReader.cs (MIT), via JosefNemec/Playnite (MIT).
public static class Bson
{
    public static Dictionary<string, object?> ReadDocument(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return ReadDocument(new LiteDbV7Reader.ByteReader(data, 0, data.Length));
    }

    internal static Dictionary<string, object?> ReadDocument(LiteDbV7Reader.ByteReader reader)
    {
        var document = new Dictionary<string, object?>(StringComparer.Ordinal);
        foreach (var (name, value) in ReadElements(reader))
        {
            document[name] = value;
        }

        return document;
    }

    private static List<(string Name, object? Value)> ReadElements(LiteDbV7Reader.ByteReader reader)
    {
        int start = reader.Position;
        int length = reader.ReadInt32();
        int end = start + length - 1;
        var elements = new List<(string, object?)>();
        while (reader.Position < end)
        {
            byte type = reader.ReadByte();
            string name = reader.ReadCString();
            elements.Add((name, ReadValue(reader, type)));
        }

        reader.Skip(1); // zero final
        return elements;
    }

    private static object? ReadValue(LiteDbV7Reader.ByteReader reader, byte type)
    {
        switch (type)
        {
            case 0x01:
                return reader.ReadDouble();
            case 0x02:
                {
                    int length = reader.ReadInt32();
                    string text = reader.ReadFixedString(Math.Max(0, length - 1));
                    reader.Skip(1);
                    return text;
                }

            case 0x03:
                return ReadDocument(reader);
            case 0x04:
                return ReadElements(reader).Select(e => e.Value).ToList();
            case 0x05:
                {
                    int length = reader.ReadInt32();
                    byte subType = reader.ReadByte();
                    byte[] bytes = reader.ReadBytes(length);
                    return subType == 0x04 && length == 16 ? new Guid(bytes) : bytes;
                }

            case 0x07:
                return reader.ReadBytes(12); // ObjectId
            case 0x08:
                return reader.ReadByte() != 0;
            case 0x09:
                {
                    long ms = reader.ReadInt64();
                    return ms switch
                    {
                        253402300800000 => DateTime.MaxValue,
                        -62135596800000 => DateTime.MinValue,
                        _ => DateTime.UnixEpoch.AddMilliseconds(ms),
                    };
                }

            case 0x0A or 0x7F or 0xFF:
                return null; // null, MaxKey, MinKey
            case 0x10:
                return reader.ReadInt32();
            case 0x12:
                return reader.ReadInt64();
            case 0x13:
                reader.Skip(16);
                return null; // decimal: nenhum campo usado tem
            default:
                throw new InvalidDataException($"Tipo BSON {type} não suportado");
        }
    }
}
