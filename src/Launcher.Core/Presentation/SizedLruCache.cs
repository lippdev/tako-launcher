namespace Launcher.Core.Presentation;

/// <summary>
/// Cache LRU limitado por custo (ex.: bytes de imagens decodificadas, PERF-004). Ao passar do limite, descarta os menos
/// usados recentemente; o item recém-adicionado sempre fica, mesmo sozinho acima do limite. Não é thread-safe: quem usa
/// sincroniza.
/// </summary>
public sealed class SizedLruCache<TKey, TValue>
    where TKey : notnull
{
    private readonly Dictionary<TKey, LinkedListNode<Entry>> _map = [];
    private readonly LinkedList<Entry> _order = new(); // primeiro = mais recente

    public SizedLruCache(long capacity)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(capacity);
        Capacity = capacity;
    }

    public long Capacity { get; }

    public long TotalCost { get; private set; }

    public int Count => _map.Count;

    public bool TryGet(TKey key, out TValue value)
    {
        if (_map.TryGetValue(key, out var node))
        {
            _order.Remove(node);
            _order.AddFirst(node);
            value = node.Value.Value;
            return true;
        }

        value = default!;
        return false;
    }

    /// <summary>Adiciona ou substitui; devolve os valores descartados para abrir espaço.</summary>
    public IReadOnlyList<TValue> Add(TKey key, TValue value, long cost)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(cost);
        Remove(key);
        var node = _order.AddFirst(new Entry(key, value, cost));
        _map[key] = node;
        TotalCost += cost;

        List<TValue>? evicted = null;
        while (TotalCost > Capacity && _order.Last is { } last && last != node)
        {
            _order.RemoveLast();
            _map.Remove(last.Value.Key);
            TotalCost -= last.Value.Cost;
            (evicted ??= []).Add(last.Value.Value);
        }

        return evicted ?? (IReadOnlyList<TValue>)[];
    }

    public bool Remove(TKey key)
    {
        if (!_map.Remove(key, out var node))
        {
            return false;
        }

        _order.Remove(node);
        TotalCost -= node.Value.Cost;
        return true;
    }

    private readonly record struct Entry(TKey Key, TValue Value, long Cost);
}
