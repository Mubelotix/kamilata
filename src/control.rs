use crate::prelude::*;

pub(crate) struct OngoingSearchState {
    queries: Vec<(Vec<String>, usize)>,
    req_limit: usize,
    timeout_ms: usize,
    queried_peers: usize,
    final_peers: usize,
    ongoing_queries: usize,
}

impl OngoingSearchState {
    pub(crate) fn new(queries: Vec<(Vec<String>, usize)>) -> OngoingSearchState {
        OngoingSearchState {
            queries,
            req_limit: 50,
            timeout_ms: 50000,
            queried_peers: 0,
            final_peers: 0,
            ongoing_queries: 0,
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

/// A search controller is an handle to an ongoing search, started with [KamilataBehaviour::search].
/// It allows getting results asynchronously, and to control the search.
pub struct OngoingSearchControler<T: SearchResult> {
    receiver: Receiver<(T, usize, PeerId)>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

pub(crate) struct OngoingSearchFollower<T: SearchResult> {
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

    /// Returns the timeout in milliseconds.
    pub async fn timeout_ms(&self) -> usize {
        self.inner.read().await.timeout_ms
    }

    /// Returns the number of peers that have been queried.
    pub async fn queried_peers(&self) -> usize {
        self.inner.read().await.queried_peers
    }

    /// Returns the number of peers that have been queried and returned local results.
    pub async fn final_peers(&self) -> usize {
        self.inner.read().await.final_peers
    }

    /// Returns the number of ongoing queries.
    pub async fn ongoing_queries(&self) -> usize {
        self.inner.read().await.ongoing_queries
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

        let inner = self.inner.read().await;

        SearchResults {
            hits: search_results,
            queried_peers: inner.queried_peers,
            final_peers: inner.final_peers,
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

    /// Returns the timeout in milliseconds.
    pub async fn timeout_ms(&self) -> usize {
        self.inner.read().await.timeout_ms
    }

    /// Sets query/peer counts.
    pub async fn set_query_counts(&self, queried_peers: usize, final_peers: usize, ongoing_queries: usize) {
        let mut inner = self.inner.write().await;
        inner.queried_peers = queried_peers;
        inner.final_peers = final_peers;
        inner.ongoing_queries = ongoing_queries;
    }
}

/// A struct containing search results and some useful information about how the search went.
#[derive(Debug)]
pub struct SearchResults<T: SearchResult> {
    /// Contains search results in the order they were received.
    /// Results that have already been [received](OngoingSearchControler::recv) are not included.
    pub hits: Vec<(T, usize, PeerId)>,
    /// Numbers of peers that have been queried
    pub queried_peers: usize,
    /// Numbers of peers that have been able to provide us with hits
    pub final_peers: usize,
}
