use std::{collections::HashMap, fmt::Write};

use tokio::sync::RwLock;

use crate::prelude::*;

pub struct Filter<const N: usize>(Box<[u8; N]>);

impl<const N: usize> Filter<N> {
    fn new() -> Self {
        Filter(Box::new([0; N]))
    }

    // TODO: unchecked version of this
    fn get_bit(&self, idx: usize) -> bool {
        if idx >= N*8 {
            return false;
        }
        let bit_idx = idx.rem_euclid(8);
        let byte_idx = idx.div_euclid(8);
        let bit = (self.0[byte_idx] >> bit_idx) & 1;
        bit != 0
    }

    fn set_bit(&mut self, idx: usize, value: bool) {
        if idx >= N*8 {
            return;
        }
        let bit_idx = idx.rem_euclid(8);
        let byte_idx = idx.div_euclid(8);
        let bit = value as u8;
        let keeping_mask = !(1 << bit_idx);
        self.0[byte_idx] = (self.0[byte_idx] & keeping_mask) + (bit << bit_idx);
    }

    fn get_all_set_bits(&self) -> Vec<usize> {
        let mut bits = Vec::new();
        for idx in 0..N*8 {
            if self.get_bit(idx) {
                bits.push(idx);
            }
        }
        bits
    }
}

impl<const N: usize> std::ops::BitOr for Filter<N> {
    type Output = Self;

    fn bitor(mut self, other: Self) -> Self::Output {
        for byte_idx in 0..N {
            unsafe {
                *self.0.get_unchecked_mut(byte_idx) |= *other.0.get_unchecked(byte_idx);
            }
        }
        self
    }
}

impl<const N: usize> std::ops::BitOrAssign for Filter<N> {
    fn bitor_assign(&mut self, other: Self) {
        for byte_idx in 0..N {
            unsafe {
                *self.0.get_unchecked_mut(byte_idx) |= *other.0.get_unchecked(byte_idx);
            }
        }
    }
}

pub struct Filters<const N: usize> {
    pub filters: RwLock<HashMap<PeerId, Vec<Filter<N>>>>,
}

impl<const N: usize> Filters<N> {
    pub fn new() -> Self {
        Filters {
            filters: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert_filters(&self, peer_id: PeerId, filters: Vec<Filter<N>>) {
        self.filters.write().await.insert(peer_id, filters);
    }

    pub async fn find_all(&self, query: Vec<usize>) -> Result<Vec<(PeerId, usize)>, &'static str> {
        // Check query
        for idx in &query {
            if *idx >= N {
                return Err("Query item is out of range");
            }
        }
        if query.is_empty() {
            return Err("Query is empty");
        }

        let mut results = Vec::new();
        'peer_loop: for (peer_id, filters) in self.filters.read().await.iter() {
            let mut found = None;
            for (distance, filter) in filters.iter().enumerate().rev() {
                for idx in &query {
                    if !filter.get_bit(*idx) {
                        if let Some(found) = found {
                            results.push((*peer_id, found));
                        }
                        continue 'peer_loop;
                    }
                }
                found = Some(distance);
            }
            if let Some(found) = found {
                results.push((*peer_id, found));
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_ops() {
        let mut filter = Filter::<4>::new();
        assert!(filter.get_all_set_bits().is_empty());
        filter.set_bit(8, true);
        assert_eq!(filter.get_all_set_bits(), vec![8]);
        filter.set_bit(8, false);
        filter.set_bit(9, true);
        assert_eq!(filter.get_all_set_bits(), vec![9]);
        filter.set_bit(10, true);
        assert_eq!(filter.get_all_set_bits(), vec![9, 10]);
    }

    #[test]
    fn or_ops() {
        let mut filter1 = Filter::<4>::new();
        filter1.set_bit(8, true);
        let mut filter2 = Filter::<4>::new();
        filter2.set_bit(9, true);
        let filter3 = filter1 | filter2;
        assert_eq!(filter3.get_all_set_bits(), vec![8, 9]);

        let mut filter1 = Filter::<4>::new();
        filter1.set_bit(8, true);
        let mut filter2 = Filter::<4>::new();
        filter2.set_bit(8, true);
        filter2.set_bit(9, true);
        let filter3 = filter1 | filter2;
        assert_eq!(filter3.get_all_set_bits(), vec![8, 9]);
    }

    #[tokio::test]
    async fn find_all_minimal() {
        let peer_id1 = PeerId::random();
        let mut filter1 = Filter::<4>::new();
        filter1.set_bit(1, true);

        let peer_id2 = PeerId::random();
        let mut filter2 = Filter::<4>::new();
        filter2.set_bit(2, true);

        let peer_id3 = PeerId::random();
        let mut filter3 = Filter::<4>::new();
        filter3.set_bit(3, true);

        let filters = Filters::<4>::new();
        filters.insert_filters(peer_id1, vec![filter1]).await;
        filters.insert_filters(peer_id2, vec![filter2]).await;
        filters.insert_filters(peer_id3, vec![filter3]).await;

        let query = vec![1];
        let results = filters.find_all(query).await.unwrap();
        assert_eq!(results, vec![(peer_id1, 0)]);
    }

    #[tokio::test]
    async fn find_all_simple() {
        let peer_id1 = PeerId::random();
        let mut filter1 = Filter::<4>::new();
        filter1.set_bit(1, true);
        filter1.set_bit(2, true);

        let peer_id2 = PeerId::random();
        let mut filter2 = Filter::<4>::new();
        filter2.set_bit(1, true);

        let peer_id3 = PeerId::random();
        let mut filter3 = Filter::<4>::new();
        filter3.set_bit(2, true);

        let filters = Filters::<4>::new();
        filters.insert_filters(peer_id1, vec![filter1]).await;
        filters.insert_filters(peer_id2, vec![filter2]).await;
        filters.insert_filters(peer_id3, vec![filter3]).await;

        let query = vec![1, 2];
        let results = filters.find_all(query).await.unwrap();
        assert_eq!(results, vec![(peer_id1, 0)]);
    }

    #[tokio::test]
    async fn find_all_multiple() {
        let peer_id1 = PeerId::random();
        let mut filter1 = Filter::<4>::new();
        filter1.set_bit(1, true);

        let peer_id2 = PeerId::random();
        let mut filter2 = Filter::<4>::new();
        filter2.set_bit(1, true);

        let peer_id3 = PeerId::random();
        let mut filter3 = Filter::<4>::new();
        filter3.set_bit(2, true);

        let filters = Filters::<4>::new();
        filters.insert_filters(peer_id1, vec![filter1]).await;
        filters.insert_filters(peer_id2, vec![filter2]).await;
        filters.insert_filters(peer_id3, vec![filter3]).await;

        let query = vec![1];
        let results = filters.find_all(query).await.unwrap();
        assert!(results.len() == 2);
        assert!(results.contains(&(peer_id1, 0)));
        assert!(results.contains(&(peer_id2, 0)));
    }

    #[tokio::test]
    async fn find_all_distances() {
        let peer_id1 = PeerId::random();
        let mut filter1_1 = Filter::<4>::new();
        filter1_1.set_bit(1, true);
        let mut filter1_2 = Filter::<4>::new();
        filter1_2.set_bit(1, true);
        filter1_2.set_bit(2, true);
        let mut filter1_3 = Filter::<4>::new();
        filter1_3.set_bit(1, true);
        filter1_3.set_bit(2, true);
        filter1_3.set_bit(3, true);

        let peer_id2 = PeerId::random();
        let mut filter2_1 = Filter::<4>::new();
        filter2_1.set_bit(1, true);
        let mut filter2_2 = Filter::<4>::new();
        filter2_2.set_bit(1, true);
        filter2_2.set_bit(2, true);
        filter2_2.set_bit(3, true);
        let mut filter2_3 = Filter::<4>::new();
        filter2_3.set_bit(1, true);
        filter2_3.set_bit(2, true);
        filter2_3.set_bit(3, true);

        let filters = Filters::<4>::new();
        filters.insert_filters(peer_id1, vec![filter1_1, filter1_2, filter1_3]).await;
        filters.insert_filters(peer_id2, vec![filter2_1, filter2_2]).await;

        let query = vec![1, 2, 3];
        let results = filters.find_all(query).await.unwrap();
        assert!(results.len() == 2);
        assert!(results.contains(&(peer_id1, 2)));
        assert!(results.contains(&(peer_id2, 1)));
    }
}
