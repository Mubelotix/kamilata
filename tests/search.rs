//! A test running a network of 50 nodes on a corpus of 32000 documents.

mod common;
use common::*;

use kamilata::prelude::*;
use libp2p::swarm::dial_opts::DialOpts;
use tokio::time::sleep;
use std::time::Duration;

const NODE_COUNT: usize = 50;

#[tokio::test]
async fn search() -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading data...");
    let data = match std::fs::read_to_string("movies.json") {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::process::Command::new("sh")
                .arg("-c")
                .arg("wget https://docs.meilisearch.com/movies.json")
                .output()
                .expect("failed to download movies.json");
            std::fs::read_to_string("movies.json").unwrap()
        },
        e => e.unwrap(),
    };
    let movies = serde_json::from_str::<Vec<Movie>>(&data).unwrap();

    info!("Initializing clients...");
    let mut clients = Vec::new();
    for i in 0..NODE_COUNT {
        let client = Client::init(i).await;
        clients.push(client);
    }

    let mut logger = ClientLogger::new();
    logger.with_peer_id(clients[0].peer_id());
    logger.activate();

    info!("Creating connections...");
    for i in 0..NODE_COUNT {
        for _ in 0..8 {
            let randint = rand::random::<usize>() % NODE_COUNT;
            let peer_id = clients[randint].peer_id();
            let addr = clients[randint].addr().to_owned();
            clients[i].swarm_mut().dial(DialOpts::peer_id(peer_id).addresses(vec![addr]).build()).unwrap();
        }
    }

    info!("Adding documents...");
    for (i, movies) in movies.as_slice().chunks((movies.len() as f64 / NODE_COUNT as f64).ceil() as usize).enumerate() {
        clients[i].behavior().insert_documents(movies.to_vec()).await;
    }

    info!("Launching clients...");
    let mut controlers = Vec::new();
    for client in clients {
        controlers.push(client.run());
    }

    info!("Waiting for 10 seconds... (to let the network stabilize)");
    sleep(Duration::from_secs(10)).await;
    
    info!("Searching...");
    let results = controlers[0].search("hunger").await;
    info!("{results:#?}");
    let mut theoretical_hits = 0;
    for movie in movies {
        if movie.words().contains(&"hunger".to_string()) {
            theoretical_hits += 1;
        }
    }
    info!("Theoretical hits: {theoretical_hits}");
    assert!(results.hits.len() == theoretical_hits);

    Ok(())
}

