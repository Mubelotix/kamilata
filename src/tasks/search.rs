//! This module contains the algorithm that is used for discovering results on the network.

use super::*;

pub async fn search<const N: usize, D: Document<N>>(
    our_peer_id: PeerId,
    search_follower: OngoingSearchFollower<D::SearchResult>,
    handler_messager: HandlerMessager,
    db: Arc<Db<N, D>>,
) -> TaskOutput {
    // Query ourselves
    let queries = search_follower.queries().await;
    let local_results = db.search_local(&queries).await;
    for (result, query) in local_results {
        search_follower.send((result, query, our_peer_id)).await.unwrap();
    }

    todo!()
}
