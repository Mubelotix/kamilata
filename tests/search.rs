//! A test running an unorganized network of many nodes, giving them all documents and then searching for them.

mod common;
use common::*;

use kamilata::prelude::*;
use libp2p::swarm::dial_opts::DialOpts;
use tokio::time::sleep;
use std::time::Duration;

const NODE_COUNT: usize = 60;

#[tokio::test]
#[ignore]
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
        for _ in 0..6 {
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
        sleep(Duration::from_millis((20000/NODE_COUNT) as u64)).await; // We launch the network over a time period of 20 seconds so that they don't always update their filters at the same time.
    }

    info!("Waiting for the network to stabilize...");
    sleep(Duration::from_secs(2)).await;
    
    info!("Searching...");
    let results = controlers[0].search("hunger").await;
    let mut expected = 0;
    let mut missing = Vec::new();
    for movie in movies {
        if movie.words().contains(&"hunger".to_string()) {
            expected += 1;
            if !results.hits.iter().any(|(r,_,_)| *r==movie) {
                missing.push(movie);
            }
        }
    }
    if missing.len() as f32 > expected as f32 * 0.15 {
        panic!("Too many missing results:\n{missing:#?}");
    } else if !missing.is_empty() {
        warn!("Less than 10% results are missing so the test is still considered successful. Missing:\n{missing:#?}");
    } else {
        info!("All results are present");
    }

    Ok(())
}

