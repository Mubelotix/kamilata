use crate::prelude::*;

pub trait SearchResult: Send + Sync + 'static {
    /// CID is required in order to detect duplicates.
    type Cid: Eq + Ord + Clone + Send + Sync + 'static;

    fn cid(&self) -> &Self::Cid;
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

pub trait Document<const N: usize>: Send + Sync + 'static {
    type SearchResult: SearchResult;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid;
    // TODO: this should be async
    fn apply_to_filter(&self, filter: &mut Filter<N>);
}

pub trait WordHasher<const N: usize> {
    fn hash_word(word: &str) -> usize;
}
