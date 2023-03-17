///! A test to make sure directives that from KamilataConfig (fields in_routing_peers and out_routing_peers) are respected.

mod common;
use common::*;

const IN_ROUTING_TARGET: usize = 5;
const OUT_ROUTING_TARGET: usize = 6;
const IN_ROUTING_MAX: usize = 7;
const OUT_ROUTING_MAX: usize = 8;

const MAIN_NODE_CONFIG: KamilataConfig = KamilataConfig {
    in_routing_peers: MinTargetMax::new(0, IN_ROUTING_TARGET, IN_ROUTING_MAX),
    out_routing_peers: MinTargetMax::new(0, OUT_ROUTING_TARGET, OUT_ROUTING_MAX),
    get_filters_interval: MinTargetMax::new(60_000_000, 60_000_000, 60_000_000),
    filter_count: 5,
};
const LAZY_NODE_CONFIG: KamilataConfig = KamilataConfig {
    in_routing_peers: MinTargetMax::new(0, 0, 999),
    out_routing_peers: MinTargetMax::new(0, 0, 999),
    get_filters_interval: MinTargetMax::new(60_000_000, 60_000_000, 60_000_000),
    filter_count: 5,
};
const GREEDY_NODE_CONFIG: KamilataConfig = KamilataConfig {
    in_routing_peers: MinTargetMax::new(0, 999, 999),
    out_routing_peers: MinTargetMax::new(0, 999, 999),
    get_filters_interval: MinTargetMax::new(60_000_000, 60_000_000, 60_000_000),
    filter_count: 5,
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
    assert_eq!(routing_stats.0, IN_ROUTING_TARGET);
    assert_eq!(routing_stats.1, OUT_ROUTING_TARGET);

    info!("Creating connections with greedy nodes...");
    for addr in greedy_client_addresses {
        main_controler.dial(addr.clone()).await;
        sleep(Duration::from_millis(500)).await;
    }

    let routing_stats = main_controler.get_routing_stats().await;
    assert_eq!(routing_stats.0, IN_ROUTING_MAX);
    assert_eq!(routing_stats.1, OUT_ROUTING_MAX);

    Ok(())
}

