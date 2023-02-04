use crate::prelude::*;

pub(crate) struct OngoingSearchState {
    queries: Vec<(Vec<String>, usize)>,
    req_limit: usize,
}

impl OngoingSearchState {
    pub(crate) fn new(queries: Vec<(Vec<String>, usize)>) -> OngoingSearchState {
        OngoingSearchState {
            queries,
            req_limit: 50,
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
    receiver: Receiver<(T, usize, PeerId)>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

pub struct OngoingSearchFollower<T: SearchResult> {
    sender: Sender<(T, usize, PeerId)>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

impl<T: SearchResult> OngoingSearchControler<T> {
    /// Waits for the next search result.
    pub async fn recv(&mut self) -> Option<(T, usize, PeerId)> {
        self.receiver.recv().await
    }

    /// Returns the next search result if available.
    pub fn try_recv(&mut self) -> Result<(T, usize, PeerId), tokio::sync::mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Returns a copy of the ongoing queries.
    pub async fn queries(&self) -> Vec<(Vec<String>, usize)> {
        self.inner.read().await.queries.clone()
    }

    /// Returns the concurrent request limit.
    pub async fn req_limit(&self) -> usize {
        self.inner.read().await.req_limit
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

impl<T: SearchResult> OngoingSearchFollower<T> {
    /// Sends a search result to the controler.
    pub async fn send(&self, search_result: (T, usize, PeerId)) -> Result<(), tokio::sync::mpsc::error::SendError<(T, usize, PeerId)>> {
        self.sender.send(search_result).await
    }

    /// Returns a copy of the ongoing queries.
    pub async fn queries(&self) -> Vec<(Vec<String>, usize)> {
        self.inner.read().await.queries.clone()
    }

    /// Returns the concurrent request limit.
    pub async fn req_limit(&self) -> usize {
        self.inner.read().await.req_limit
    }
}

pub struct SearchResults<T: SearchResult> {
    /// Contains search results in the order they were received.
    /// Results that have already been [received](OngoingSearchControler::recv) are not included.
    search_results: Vec<(T, usize, PeerId)>,
}
