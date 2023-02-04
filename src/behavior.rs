use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataEvent {

}

/// A struct that allows to send messages to an [handler](ConnectionHandler)
pub struct HandlerMessager {
    sender: Sender<(PeerId, HandlerInEvent)>,
}

impl HandlerMessager {
    /// Sends a message to the handler.
    pub async fn message(&self, peer_id: PeerId, message: HandlerInEvent) {
        self.sender.send((peer_id, message)).await.unwrap();
    }
}

pub struct KamilataBehavior<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    db: Arc<Db<N, D>>,
    handler_event_sender: Sender<(PeerId, HandlerInEvent)>,
    handler_event_receiver: Receiver<(PeerId, HandlerInEvent)>,
    
    rt_handle: tokio::runtime::Handle,

    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     none
    tasks: HashMap<usize, Task>,
}

impl<const N: usize, D: Document<N>> KamilataBehavior<N, D> {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior<N, D> {
        let rt_handle = tokio::runtime::Handle::current();
        let (handler_event_sender, handler_event_receiver) = channel(100);

        KamilataBehavior {
            our_peer_id,
            db: Arc::new(Db::new()),
            handler_event_sender,
            handler_event_receiver,
            rt_handle,
            task_counter: Counter::new(0),
            tasks: HashMap::new(),
        }
    }

    pub async fn insert_document(&self, document: D) {
        self.db.insert_document(document).await;
    }

    pub async fn insert_documents(&self, documents: Vec<D>) {
        self.db.insert_documents(documents).await;
    }

    pub async fn clear_documents(&self) {
        self.db.clear_documents().await;
    }

    pub async fn remove_document(&self, cid: &<D::SearchResult as SearchResult>::Cid) {
        self.db.remove_document(cid).await;
    }

    pub async fn remove_documents(&self, cids: &[&<D::SearchResult as SearchResult>::Cid]) {
        self.db.remove_documents(cids).await;
    }

    /// Starts a new search and returns an [handler](OngoingSearchControler) to control it.
    pub async fn search(&mut self, words: Vec<String>) -> OngoingSearchControler<D::SearchResult> {
        let handler_messager = HandlerMessager {
            sender: self.handler_event_sender.clone(),
        };
        let words_len = words.len();
        let search_state = OngoingSearchState::new(vec![(words, words_len)]);
        let (search_controler, search_follower) = search_state.into_pair::<D::SearchResult>();
        self.tasks.insert(self.task_counter.next() as usize, Box::pin(search(self.our_peer_id, search_follower, handler_messager, Arc::clone(&self.db))));
        search_controler
    }
}

impl<const N: usize, D: Document<N>> NetworkBehaviour for KamilataBehavior<N, D> {
    type ConnectionHandler = KamilataHandlerProto<N, D>;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: HandlerOutEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        // Message handlers first
        if let Poll::Ready(Some((peer_id, event))) = self.handler_event_receiver.poll_recv(cx) {
            return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                handler: libp2p::swarm::NotifyHandler::Any,
                event
            })
        }

        // It seems this method gets called in a context where the tokio runtime does not exist.
        // We import that runtime so that we can rely on it.
        let _rt_enter_guard = self.rt_handle.enter();

        // Poll tasks
        for tid in self.tasks.keys().copied().collect::<Vec<_>>() {
            let task = self.tasks.get_mut(&tid).unwrap();

            match task.poll_unpin(cx) {
                Poll::Ready(output) => {
                    println!("{} Task {tid} completed!", self.our_peer_id);
                    self.tasks.remove(&tid);

                    match output {
                        TaskOutput::None => (),
                    }
                }
                Poll::Pending => ()
            }
        }
        
        Poll::Pending
    }
}
