use crate::prelude::*;

pub(crate) struct OngoingSearchState {
    queries: Vec<Vec<String>>,
}

impl OngoingSearchState {
    pub(crate) fn new(queries: Vec<Vec<String>>) -> OngoingSearchState {
        OngoingSearchState {
            queries,
        }
    }

    pub(crate) fn into_pair<T: SearchResult>(self) -> (OngoingSearchControler<T>, OngoingSearchFollower<T>) {
        let (sender, receiver) = channel(100);
        let inner = Arc::new(RwLock::new(self));

        (
            OngoingSearchControler {
                receiver,
                inner: inner.clone(),
            },
            OngoingSearchFollower {
                sender,
                inner,
            },
        )
    }
}

pub struct OngoingSearchControler<T: SearchResult> {
    receiver: Receiver<T>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

pub struct OngoingSearchFollower<T: SearchResult> {
    sender: Sender<T>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

impl<T: SearchResult> OngoingSearchControler<T> {
    /// Waits for the next search result.
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }

    /// Returns the next search result if available.
    pub fn try_recv(&mut self) -> Result<T, tokio::sync::mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Returns a copy of the ongoing queries.
    pub async fn queries(&self) -> Vec<Vec<String>> {
        self.inner.read().await.queries.clone()
    }

    /// Truncates the ongoing queries to only keep the most important ones.
    /// This is useful when you start having enough relevant results to stop searching for less relevant ones.
    pub async fn truncate_queries(&self, len: usize) {
        self.inner.write().await.queries.truncate(len)
    }

    /// Stops the search and returns all search results that have not been consumed yet.
    pub async fn finish(mut self) -> SearchResults<T> {
        let mut search_results = Vec::new();
        while let Ok(search_result) = self.try_recv() {
            search_results.push(search_result);
        }

        SearchResults {
            search_results,
        }
    }
}

pub struct SearchResults<T: SearchResult> {
    /// Contains search results in the order they were received.
    /// Results that have already been [received](OngoingSearchControler::recv) are not included.
    search_results: Vec<T>,
}
