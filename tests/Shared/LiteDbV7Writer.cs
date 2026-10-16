using System.Buffers.Binary;
using System.Text;

namespace Launcher.Tests;

/// <summary>
/// Gera arquivos no formato do LiteDB 4 ("v7", páginas de 4 KB) só para os testes da importação do Playnite (STORE-009):
/// cabeçalho, uma página de coleção, o índice <c>_id</c>, páginas de dados e, para documentos grandes, a corrente de
/// páginas de extensão. Não é um LiteDB completo; produz o mínimo que o Playnite grava e o leitor do Tako lê.
/// </summary>
public static class LiteDbV7Writer
{
    private const int PageSize = 4096;
    private const int HeaderSize = 25;
    private const uint Empty = uint.MaxValue;

    /// <summary>Um arquivo com uma coleção por par nome → documentos.</summary>
    public static byte[] Write(IReadOnlyDictionary<string, IReadOnlyList<Dictionary<string, object?>>> collections, int inlineLimit = 2000)
    {
        var pages = new List<byte[]> { new byte[PageSize] };
        var collectionPages = new Dictionary<string, uint>();
        foreach (var (name, documents) in collections)
        {
            uint collectionPage = Add(pages);
            uint indexPage = Add(pages);
            collectionPages[name] = collectionPage;

            // Documentos: blocos em páginas de dados (vários por página); os grandes vão para páginas de extensão.
            var locations = new List<(uint Page, ushort Index)>();
            uint dataPage = Empty;
            int used = 0;
            ushort count = 0;
            var blocks = new List<byte[]>();
            void Flush()
            {
                if (dataPage != Empty)
                {
                    WritePage(pages[(int)dataPage], dataPage, 4, Empty, (ushort)blocks.Count, blocks.SelectMany(b => b).ToArray());
                }
            }

            foreach (var document in documents)
            {
                byte[] bson = Bson(document);
                byte[] inline = bson.Length <= inlineLimit ? bson : [];
                int blockSize = 2 + 4 + 2 + inline.Length;
                if (dataPage == Empty || used + blockSize > PageSize - HeaderSize)
                {
                    Flush();
                    dataPage = Add(pages);
                    used = 0;
                    count = 0;
                    blocks = [];
                }

                uint extend = bson.Length <= inlineLimit ? Empty : WriteExtend(pages, bson);
                var block = new byte[blockSize];
                BinaryPrimitives.WriteUInt16LittleEndian(block, count);
                BinaryPrimitives.WriteUInt32LittleEndian(block.AsSpan(2), extend);
                BinaryPrimitives.WriteUInt16LittleEndian(block.AsSpan(6), (ushort)inline.Length);
                inline.CopyTo(block, 8);
                blocks.Add(block);
                locations.Add((dataPage, count));
                used += blockSize;
                count++;
            }

            Flush();

            // Índice _id: nó de cabeça (sem dados) + um nó por documento, todos na mesma página.
            var nodes = new List<byte>();
            var all = new List<(uint Page, ushort Index)> { (Empty, 0) };
            all.AddRange(locations);
            for (int i = 0; i < all.Count; i++)
            {
                var node = new byte[37];
                BinaryPrimitives.WriteUInt16LittleEndian(node, (ushort)i);
                node[2] = 1; // níveis
                BinaryPrimitives.WriteUInt16LittleEndian(node.AsSpan(16), 0); // tamanho da chave
                BinaryPrimitives.WriteUInt32LittleEndian(node.AsSpan(19), all[i].Page);
                BinaryPrimitives.WriteUInt16LittleEndian(node.AsSpan(23), all[i].Index);
                BinaryPrimitives.WriteUInt32LittleEndian(node.AsSpan(25), i == 0 ? Empty : indexPage);
                BinaryPrimitives.WriteUInt16LittleEndian(node.AsSpan(29), (ushort)Math.Max(0, i - 1));
                BinaryPrimitives.WriteUInt32LittleEndian(node.AsSpan(31), i == all.Count - 1 ? Empty : indexPage);
                BinaryPrimitives.WriteUInt16LittleEndian(node.AsSpan(35), (ushort)(i + 1));
                nodes.AddRange(node);
            }

            WritePage(pages[(int)indexPage], indexPage, 3, Empty, (ushort)all.Count, [.. nodes]);

            // Página da coleção: nome, 12 bytes e 16 índices (o primeiro é o _id).
            var body = new List<byte>();
            body.AddRange(LengthString(name));
            body.AddRange(new byte[12]);
            for (int i = 0; i < 16; i++)
            {
                body.AddRange(LengthString(i == 0 ? "_id" : string.Empty));
                body.Add(i == 0 ? (byte)1 : (byte)0);
                var head = new byte[4];
                BinaryPrimitives.WriteUInt32LittleEndian(head, i == 0 ? indexPage : Empty);
                body.AddRange(head);
                body.AddRange(new byte[2 + 6 + 4]);
            }

            WritePage(pages[(int)collectionPage], collectionPage, 2, Empty, 0, [.. body]);
        }

        // Cabeçalho do arquivo.
        var header = new List<byte>();
        header.AddRange(Encoding.UTF8.GetBytes("** This is a LiteDB file **"));
        header.Add(7);
        header.AddRange(new byte[2 + 4 + 4 + 2 + 20 + 16]);
        header.Add((byte)collectionPages.Count);
        foreach (var (name, page) in collectionPages)
        {
            header.AddRange(LengthString(name));
            var id = new byte[4];
            BinaryPrimitives.WriteUInt32LittleEndian(id, page);
            header.AddRange(id);
        }

        WritePage(pages[0], 0, 1, Empty, 0, [.. header]);
        return [.. pages.SelectMany(p => p)];
    }

    /// <summary>Um documento em BSON (texto, número, booleano, data, Guid, lista, documento e nulo).</summary>
    public static byte[] Bson(IReadOnlyDictionary<string, object?> document)
    {
        var body = new List<byte>();
        foreach (var (name, value) in document)
        {
            Element(body, name, value);
        }

        return Wrap(body);
    }

    private static byte[] Wrap(List<byte> body)
    {
        var result = new byte[body.Count + 5];
        BinaryPrimitives.WriteInt32LittleEndian(result, result.Length);
        body.CopyTo(result, 4);
        return result;
    }

    private static void Element(List<byte> body, string name, object? value)
    {
        byte type = value switch
        {
            null => 0x0A,
            double => 0x01,
            string => 0x02,
            IReadOnlyDictionary<string, object?> => 0x03,
            System.Collections.IEnumerable => 0x04,
            Guid => 0x05,
            bool => 0x08,
            DateTime => 0x09,
            int => 0x10,
            long => 0x12,
            _ => throw new NotSupportedException(value.GetType().Name),
        };
        body.Add(type);
        body.AddRange(Encoding.UTF8.GetBytes(name));
        body.Add(0);
        switch (value)
        {
            case double d:
                body.AddRange(BitConverter.GetBytes(d));
                break;
            case string s:
                byte[] text = Encoding.UTF8.GetBytes(s);
                body.AddRange(BitConverter.GetBytes(text.Length + 1));
                body.AddRange(text);
                body.Add(0);
                break;
            case IReadOnlyDictionary<string, object?> document:
                body.AddRange(Bson(document));
                break;
            case Guid g:
                body.AddRange(BitConverter.GetBytes(16));
                body.Add(0x04);
                body.AddRange(g.ToByteArray());
                break;
            case bool b:
                body.Add(b ? (byte)1 : (byte)0);
                break;
            case DateTime date:
                body.AddRange(BitConverter.GetBytes((long)(date.ToUniversalTime() - DateTime.UnixEpoch).TotalMilliseconds));
                break;
            case int i:
                body.AddRange(BitConverter.GetBytes(i));
                break;
            case long l:
                body.AddRange(BitConverter.GetBytes(l));
                break;
            case System.Collections.IEnumerable list:
                var items = new List<byte>();
                int index = 0;
                foreach (object? item in list)
                {
                    Element(items, (index++).ToString(System.Globalization.CultureInfo.InvariantCulture), item);
                }

                body.AddRange(Wrap(items));
                break;
        }
    }

    private static uint WriteExtend(List<byte[]> pages, byte[] data)
    {
        const int capacity = PageSize - HeaderSize;
        var ids = new List<uint>();
        for (int offset = 0; offset < data.Length; offset += capacity)
        {
            ids.Add(Add(pages));
        }

        for (int i = 0; i < ids.Count; i++)
        {
            int offset = i * capacity;
            int length = Math.Min(capacity, data.Length - offset);
            WritePage(pages[(int)ids[i]], ids[i], 5, i == ids.Count - 1 ? Empty : ids[i + 1], (ushort)length, data[offset..(offset + length)]);
        }

        return ids[0];
    }

    private static uint Add(List<byte[]> pages)
    {
        pages.Add(new byte[PageSize]);
        return (uint)(pages.Count - 1);
    }

    private static void WritePage(byte[] page, uint id, byte type, uint next, ushort items, byte[] body)
    {
        BinaryPrimitives.WriteUInt32LittleEndian(page, id);
        page[4] = type;
        BinaryPrimitives.WriteUInt32LittleEndian(page.AsSpan(5), Empty);
        BinaryPrimitives.WriteUInt32LittleEndian(page.AsSpan(9), next);
        BinaryPrimitives.WriteUInt16LittleEndian(page.AsSpan(13), items);
        body.CopyTo(page, HeaderSize);
    }

    private static byte[] LengthString(string text)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(text);
        var result = new byte[4 + bytes.Length];
        BinaryPrimitives.WriteInt32LittleEndian(result, bytes.Length);
        bytes.CopyTo(result, 4);
        return result;
    }
}
