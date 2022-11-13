use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataEvent {

}

pub struct KamilataBehavior<D: Document> {
    our_peer_id: PeerId,
    db: Arc<Db<D>>
}

impl<D: Document> NetworkBehaviour for KamilataBehavior<D> {
    type ConnectionHandler = KamilataHandlerProto<D>;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        Poll::Pending
    }
}

impl<D: Document> KamilataBehavior<D> {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior<D> {
        KamilataBehavior {
            our_peer_id,
            db: Arc::new(Db::new())
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
}
