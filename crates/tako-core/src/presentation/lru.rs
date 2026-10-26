//! Port de `Presentation/SizedLruCache.cs` (PERF-004): cache LRU limitado por custo (ex.: bytes de imagens
//! decodificadas). Passando do limite, descarta os menos usados; o recém-adicionado sempre fica, mesmo sozinho acima
//! do limite. Não é thread-safe: quem usa sincroniza.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

pub struct SizedLruCache<K, V> {
    capacity: u64,
    total_cost: u64,
    tick: u64,
    map: HashMap<K, (V, u64, u64)>, // valor, custo, último uso
    order: BTreeMap<u64, K>,        // último uso → chave (menor = mais antigo)
}

impl<K: Hash + Eq + Clone, V> SizedLruCache<K, V> {
    pub fn new(capacity: u64) -> Self {
        SizedLruCache {
            capacity,
            total_cost: 0,
            tick: 0,
            map: HashMap::new(),
            order: BTreeMap::new(),
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn total_cost(&self) -> u64 {
        self.total_cost
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Lê e marca como usado agora.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.tick += 1;
        let tick = self.tick;
        let entry = self.map.get_mut(key)?;
        self.order.remove(&entry.2);
        self.order.insert(tick, key.clone());
        entry.2 = tick;
        Some(&entry.0)
    }

    /// Adiciona ou substitui; devolve os valores descartados para abrir espaço.
    pub fn add(&mut self, key: K, value: V, cost: u64) -> Vec<V> {
        self.remove(&key);
        self.tick += 1;
        self.order.insert(self.tick, key.clone());
        self.map.insert(key, (value, cost, self.tick));
        self.total_cost += cost;

        let mut evicted = Vec::new();
        while self.total_cost > self.capacity && self.map.len() > 1 {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some((v, c, _)) = self.map.remove(&oldest) {
                self.total_cost -= c;
                evicted.push(v);
            }
        }
        evicted
    }

    pub fn remove(&mut self, key: &K) -> bool {
        match self.map.remove(key) {
            Some((_, cost, tick)) => {
                self.order.remove(&tick);
                self.total_cost -= cost;
                true
            }
            None => false,
        }
    }

    /// Esvazia o cache (ex.: "Limpar cache de imagens").
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.total_cost = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_until_under_capacity() {
        let mut cache = SizedLruCache::new(10);
        cache.add("a", "A", 4);
        cache.add("b", "B", 4);
        assert!(cache.get(&"a").is_some()); // "a" vira o mais recente

        assert_eq!(cache.add("c", "C", 4), ["B"]);
        assert!(cache.get(&"b").is_none());
        assert_eq!(cache.get(&"a"), Some(&"A"));
        assert_eq!(cache.total_cost(), 8);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn keeps_the_newest_item_even_above_capacity() {
        let mut cache = SizedLruCache::new(10);
        cache.add(1, 1, 3);
        assert_eq!(cache.add(2, 2, 50), [1]);
        assert!(cache.get(&2).is_some());
        assert_eq!(cache.total_cost(), 50);
    }

    #[test]
    fn replacing_a_key_updates_its_cost() {
        let mut cache = SizedLruCache::new(100);
        cache.add(1, "x", 10);
        cache.add(1, "y", 30);
        assert_eq!(cache.total_cost(), 30);
        assert_eq!(cache.len(), 1);
        assert!(cache.remove(&1));
        assert_eq!(cache.total_cost(), 0);
        assert!(!cache.remove(&1));
        cache.add(2, "z", 1);
        cache.clear();
        assert!(cache.is_empty());
    }
}
