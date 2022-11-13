use crate::prelude::*;

pub trait SearchResult {
    type Cid: Eq;

    fn cid(&self) -> &Self::Cid;
    fn into_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

pub trait Document {
    type SearchResult: SearchResult;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid;
    fn apply_to_filter(&self, filter: &mut Filter<125000>);
}
