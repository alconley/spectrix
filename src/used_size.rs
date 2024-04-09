use std::collections::BTreeMap;
pub trait UsedSize {
    fn get_used_size(&self) -> usize;
}

impl<K: UsedSize, V: UsedSize> UsedSize for BTreeMap<K, V> {
    fn get_used_size(&self) -> usize {
        let element_size: usize = std::mem::size_of::<K>() + std::mem::size_of::<V>();
        let directly_owned = self.len() * element_size;
        let transitively_owned: usize = self
            .iter()
            .map(|(key, val)| key.get_used_size() + val.get_used_size())
            .sum();
        directly_owned + transitively_owned
    }
}

impl<T: UsedSize> UsedSize for Vec<T> {
    fn get_used_size(&self) -> usize {
        self.len() * std::mem::size_of::<T>()
    }
}

impl UsedSize for i32 {
    fn get_used_size(&self) -> usize {
        std::mem::size_of::<i32>()
    }
}

impl UsedSize for f64 {
    fn get_used_size(&self) -> usize {
        std::mem::size_of::<f64>()
    }
}
