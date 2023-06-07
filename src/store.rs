use crate::prelude::*;
use async_trait::async_trait;

pub trait SearchResult: Send + Sync + 'static {
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

#[async_trait]
pub trait Store<const N: usize>: Send + Sync + 'static {
    type SearchResult: SearchResult + Send + Sync + 'static;
    
    fn hash_word(word: &str) -> usize;
    async fn get_filter(&self) -> Filter<N>; // TODO: use reference?

    // expanded async search
    fn search(&self, words: &[String], min_matching: usize) -> Pin<Box<dyn Future<Output = Vec<Self::SearchResult>> + Send + Sync + 'static>>;
}
