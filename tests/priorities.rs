//! These tests are for checking that documents are found in accordance with the priority settings.
//! In both tests, two documents are added to the network:
//!   - A perfectly matching document at a distance of 2
//!   - A partially matching document at a distance of 1
//! Depending on the priority, the order of the results should be different.

mod common;
use common::*;

#[tokio::test]
async fn speed_priority() {
    let doc1 = Movie {
        id: 0,
        title: String::from("Perfect match"),
        overview: String::from("This is the perfectly matching document"),
        genres: Vec::new(),
        poster: String::new(),
        release_date: 0,
    };
    let doc2 = Movie {
        id: 1,
        title: String::from("Partial match"),
        overview: String::from("This is the partially matching document"),
        genres: Vec::new(),
        poster: String::new(),
        release_date: 0,
    };

    let mut client1 = Client::init(1).await; // Connected to 2 and 4
    let mut client2 = Client::init(2).await; // Connected to 1 and 3
    let mut client3 = Client::init(3).await; // Distance from 1: 2
    let mut client4 = Client::init(4).await; // Distance from 1: 1

    let mut logger = ClientLogger::new();
    logger.with_peer_id(client1.peer_id());
    logger.activate();

    client1.swarm_mut().dial(DialOpts::peer_id(client2.peer_id()).addresses(vec![client2.addr().to_owned()]).build()).unwrap();
    client1.swarm_mut().dial(DialOpts::peer_id(client4.peer_id()).addresses(vec![client4.addr().to_owned()]).build()).unwrap();
    client2.swarm_mut().dial(DialOpts::peer_id(client3.peer_id()).addresses(vec![client3.addr().to_owned()]).build()).unwrap();

    client3.behavior_mut().insert_document(doc1.clone()).await;
    client4.behavior_mut().insert_document(doc2.clone()).await;

    let controler1 = client1.run();
    let controler2 = client2.run();
    let controler3 = client3.run();
    let controler4 = client4.run();

    info!("Waiting for filters to propagate...");
    sleep(Duration::from_secs(65)).await;

    info!("Searching...");
    let results = controler1.search(vec!["perfect match", "match"]).await;
    info!("Results: {:?}", results);
}
