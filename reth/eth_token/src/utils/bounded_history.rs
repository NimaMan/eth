use std::hash::Hash;

use indexmap::IndexMap;

pub fn append_with_history_limit<T>(items: &mut Vec<T>, entry: T, limit: usize) {
    items.push(entry);
    trim_vec_to_limit(items, limit);
}

pub fn append_to_index_map_history<K, V>(
    mapping: &mut IndexMap<K, Vec<V>>,
    key: K,
    entry: V,
    limit: usize,
) where
    K: Eq + Hash + Clone,
{
    let bucket = mapping.entry(key.clone()).or_default();
    bucket.push(entry);
    trim_vec_to_limit(bucket, limit);

    while mapping.len() > limit {
        if mapping.len() == 1 && mapping.contains_key(&key) {
            break;
        }
        mapping.shift_remove_index(0);
    }
}

fn trim_vec_to_limit<T>(items: &mut Vec<T>, limit: usize) {
    if limit == 0 {
        items.clear();
        return;
    }

    if items.len() > limit {
        let excess = items.len() - limit;
        items.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_with_history_limit_trims_oldest_entries() {
        let mut items = vec![1, 2];

        append_with_history_limit(&mut items, 3, 2);

        assert_eq!(items, vec![2, 3]);
    }

    #[test]
    fn append_to_index_map_history_trims_bucket_and_oldest_keys() {
        let mut mapping = IndexMap::new();

        append_to_index_map_history(&mut mapping, "a", 1, 2);
        append_to_index_map_history(&mut mapping, "a", 2, 2);
        append_to_index_map_history(&mut mapping, "a", 3, 2);
        append_to_index_map_history(&mut mapping, "b", 4, 2);
        append_to_index_map_history(&mut mapping, "c", 5, 2);

        assert!(!mapping.contains_key("a"));
        assert_eq!(mapping.get("b"), Some(&vec![4]));
        assert_eq!(mapping.get("c"), Some(&vec![5]));
    }
}
