use crate::prelude::*;

pub trait SearchResult: Send + Sync + std::fmt::Debug + 'static {
    /// CID is required in order to detect duplicates.
    type Cid: Eq + Ord + Clone + Send + Sync + 'static + std::fmt::Debug;

    fn cid(&self) -> &Self::Cid;
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

pub trait Document<const N: usize>: Send + Sync + std::fmt::Debug + 'static {
    type SearchResult: SearchResult;
    type WordHasher: WordHasher<N>;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid;
    // TODO: this should be async
    fn apply_to_filter(&self, filter: &mut Filter<N>);
    fn search_result(&self, words: &[String], min_matching: usize) -> Option<Self::SearchResult>;
}

pub trait WordHasher<const N: usize> {
    fn hash_word(word: &str) -> usize;
}
