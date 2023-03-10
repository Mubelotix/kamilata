use crate::prelude::*;

/// Primitive type representing a search priority, used to determine which results to load first.
#[derive(Debug, Clone)]
pub enum FixedSearchPriority {
    /// Get results fast without taking relevance into account
    Speed,
    /// Focus on finding the best results first
    Relevance
}

/// Advanced search priority that can change over time based on variable parameters
#[derive(Debug, Clone)]
pub enum SearchPriority {
    /// Simple search priority that cannot change automatically
    Fixed(FixedSearchPriority),
    /// Variable search priority that can change over time
    /// 
    /// Note: If you need more control, remember you can programmatically change the priority using [OngoingSearchController::set_priority].
    Variable {
        /// What priority to use first
        first: Box<SearchPriority>,
        /// Until how many documents have been found
        until_documents: usize,
        /// What priority to use after
        then: Box<SearchPriority>,
    }
}

impl SearchPriority {
    /// Builds a priority fixed to [FixedSearchPriority::Speed]
    pub fn speed() -> SearchPriority {
        SearchPriority::Fixed(FixedSearchPriority::Speed)
    }

    /// Builds a priority fixed to [FixedSearchPriority::Relevance]
    pub fn relevance() -> SearchPriority {
        SearchPriority::Fixed(FixedSearchPriority::Relevance)
    }

    /// Determines the current [FixedSearchPriority] based on parameters
    pub fn get_priority(&self, documents_found: usize) -> FixedSearchPriority {
        match self {
            SearchPriority::Fixed(p) => p.clone(),
            SearchPriority::Variable { first, until_documents, then } => {
                if documents_found < *until_documents {
                    first.get_priority(documents_found)
                } else {
                    then.get_priority(documents_found)
                }
            }
        }
    }
}

/// Configuration for a search
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Priority of the search
    pub priority: SearchPriority,
    /// Maximum number of concurrent requests to send to peers
    pub req_limit: usize,
    /// Number of milliseconds to wait for a response before considering the peer unresponsive
    pub timeout_ms: usize,
}

impl SearchConfig {
    pub fn new(priority: SearchPriority, req_limit: usize, timeout_ms: usize) -> SearchConfig {
        SearchConfig {
            priority,
            req_limit,
            timeout_ms,
        }
    }

    pub fn with_priority(self, priority: SearchPriority) -> SearchConfig {
        SearchConfig {
            priority,
            ..self
        }
    }

    pub fn with_req_limit(self, req_limit: usize) -> SearchConfig {
        // TODO: req_limit should be at least 1
        SearchConfig {
            req_limit,
            ..self
        }
    }

    pub fn with_timeout_ms(self, timeout_ms: usize) -> SearchConfig {
        SearchConfig {
            timeout_ms,
            ..self
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            priority: SearchPriority::Variable {
                first: Box::new(SearchPriority::speed()),
                until_documents: 25,
                then: Box::new(SearchPriority::relevance()),
            },
            req_limit: 10,
            timeout_ms: 50000,
        }
    }
}

pub(crate) struct OngoingSearchState {
    queries: SearchQueries,
    config: SearchConfig,
    queried_peers: usize,
    final_peers: usize,
    ongoing_queries: usize,
}

impl OngoingSearchState {
    pub(crate) fn new(queries: SearchQueries, config: SearchConfig) -> OngoingSearchState {
        OngoingSearchState {
            queries,
            config,
            queried_peers: 0,
            final_peers: 0,
            ongoing_queries: 0,
        }
    }

    pub(crate) fn into_pair<T: SearchResult>(self) -> (OngoingSearchController<T>, OngoingSearchFollower<T>) {
        let (sender, receiver) = channel(100);
        let inner = Arc::new(RwLock::new(self));

        (
            OngoingSearchController {
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
pub struct OngoingSearchController<T: SearchResult> {
    receiver: Receiver<(T, usize, PeerId)>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

pub(crate) struct OngoingSearchFollower<T: SearchResult> {
    sender: Sender<(T, usize, PeerId)>,
    inner: Arc<RwLock<OngoingSearchState>>,
}

impl<T: SearchResult> OngoingSearchController<T> {
    /// Waits for the next search result.
    pub async fn recv(&mut self) -> Option<(T, usize, PeerId)> {
        self.receiver.recv().await
    }

    /// Returns the next search result if available.
    pub fn try_recv(&mut self) -> Result<(T, usize, PeerId), tokio::sync::mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Returns a copy of the ongoing queries.
    pub async fn queries(&self) -> SearchQueries {
        self.inner.read().await.queries.clone()
    }

    /// Returns the current search config.
    pub async fn config(&self) -> SearchConfig {
        self.inner.read().await.config.clone()
    }

    /// Sets the search config.
    pub async fn set_config(&self, config: SearchConfig) {
        self.inner.write().await.config = config;
    }

    /// Returns the current search priority.
    pub async fn priority(&self) -> SearchPriority {
        self.inner.read().await.config.priority.clone()
    }

    /// Sets the search priority.
    pub async fn set_priority(&self, priority: SearchPriority) {
        self.inner.write().await.config.priority = priority;
    }

    /// Returns the concurrent request limit.
    pub async fn req_limit(&self) -> usize {
        self.inner.read().await.config.req_limit
    }

    /// Returns the timeout in milliseconds.
    pub async fn timeout_ms(&self) -> usize {
        self.inner.read().await.config.timeout_ms
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
    pub async fn queries(&self) -> SearchQueries {
        self.inner.read().await.queries.clone()
    }

    /// Returns the current search config.
    pub async fn config(&self) -> SearchConfig {
        self.inner.read().await.config.clone()
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
