use crate::prelude::*;

pub trait SearchResult: Send + Sync + 'static {
    type Cid: Eq + Ord + Clone + Send + Sync + 'static;

    fn cid(&self) -> &Self::Cid;
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

pub trait Document: Send + Sync + 'static {
    type SearchResult: SearchResult;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid;
    fn apply_to_filter(&self, filter: &mut Filter<125000>);
}
