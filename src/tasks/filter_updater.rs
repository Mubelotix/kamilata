//! This module contains the task responsible for updating our filters periodically.

use super::*;

pub async fn update_our_filters<const N: usize, D: Document<N>>(db: Arc<Db<N, D>>, our_peer_id: PeerId) -> TaskOutput {
    loop {
        db.update_our_filters().await;
        trace!("{our_peer_id} Updated our filters");
        sleep(Duration::from_secs(20)).await; // TODO config
    }
}
