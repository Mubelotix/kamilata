use crate::prelude::*;
use async_trait::async_trait;

pub trait SearchResult: Send + Sync + 'static {
    type Cid: std::hash::Hash + Eq + Send + Sync + std::fmt::Debug;

    fn cid(&self) -> Self::Cid;
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

/// This library lets you manage your documents the way you want.
/// This trait must be implemented on your document store.
#[async_trait]
pub trait Store<const N: usize>: Send + Sync + Sized + 'static {
    type SearchResult: SearchResult + Send + Sync + 'static;
    
    /// Hash a word the way you like.
    /// Must return values lower than `N*8` as they will be used as bit indices in filters.
    fn hash_word(word: &str) -> usize;

    /// Return a filter that has been filled with the words of the documents.
    /// This function is intented to return a cached value as the filter should have been generated earlier.
    async fn get_filter(&self) -> Filter<N>; // TODO: use reference?

    /// Search among all documents and return those matching at least `min_matching` words.
    fn search(&self, words: Vec<String>, min_matching: usize) -> Pin<Box<dyn Future<Output = Vec<Self::SearchResult>> + Send + Sync + 'static>>;
}
