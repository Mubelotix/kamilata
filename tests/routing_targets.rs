///! A test to make sure directives that from KamilataConfig (fields in_routing_peers and out_routing_peers) are respected.

mod common;
use common::*;

const MAIN_NODE_CONFIG: KamilataConfig = KamilataConfig {
    max_seeders: 5,
    max_leechers: 5,
    get_filters_interval: MinTargetMax::new(60_000_000, 60_000_000, 60_000_000),
    filter_count: 5,
    auto_leech: true,
};
const LAZY_NODE_CONFIG: KamilataConfig = KamilataConfig {
    max_seeders: 4,
    max_leechers: 4,
    ..MAIN_NODE_CONFIG
};
const GREEDY_NODE_CONFIG: KamilataConfig = KamilataConfig {
    max_seeders: 6,
    max_leechers: 6,
    ..MAIN_NODE_CONFIG
};

#[tokio::test]
async fn routing_targets() -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing clients...");
    let main_client = Client::init_with_config(MAIN_NODE_CONFIG).await;
    let mut clients = Vec::new();
    for _ in 0..10 {
        let client = Client::init_with_config(LAZY_NODE_CONFIG).await;
        clients.push(client);
    }
    for _ in 0..5 {
        let client = Client::init_with_config(GREEDY_NODE_CONFIG).await;
        clients.push(client);
    }

    let mut logger = ClientLogger::new();
    logger.with_peer_id(main_client.peer_id());
    logger.activate();

    info!("Launching clients...");
    let main_controler = main_client.run();
    let mut controlers = Vec::new();
    let mut addresses = Vec::new();
    for client in clients {
        addresses.push(client.addr().clone());
        controlers.push(client.run());
    }
    let (lazy_client_addresses, greedy_client_addresses) = addresses.split_at(10);

    info!("Creating connections with lazy nodes...");
    for addr in lazy_client_addresses {
        main_controler.dial(addr.clone()).await;
        sleep(Duration::from_millis(500)).await;
    }

    let routing_stats = main_controler.get_routing_stats().await;
    assert_eq!(routing_stats.0, 5);
    assert_eq!(routing_stats.1, 5);

    info!("Creating connections with greedy nodes...");
    for addr in greedy_client_addresses {
        main_controler.dial(addr.clone()).await;
        sleep(Duration::from_millis(500)).await;
    }

    let routing_stats = main_controler.get_routing_stats().await;
    assert_eq!(routing_stats.0, 5);
    assert_eq!(routing_stats.1, 5);

    Ok(())
}

