pub mod actix_server;
pub mod extension;
pub mod server;

use rusted_cypher::GraphClient;
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
use warpgrapher::client::WarpgrapherClient;
use warpgrapher::engine::config::Config;

#[allow(dead_code)]
pub fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[allow(dead_code)]
pub fn db_url() -> String {
    match var_os("DB_URL") {
        None => "http://neo4j:testpass@127.0.0.1:7474/db/data".to_owned(),
        Some(os) => os
            .to_str()
            .unwrap_or("http://neo4j:testpass@127.0.0.1:7474/db/data")
            .to_owned(),
    }
}

#[allow(dead_code)]
pub fn server_addr() -> String {
    let port = match var_os("WG_SAMPLE_PORT") {
        None => 5000,
        Some(os) => os.to_str().unwrap_or("5000").parse::<u16>().unwrap_or(5000),
    };

    format!("127.0.0.1:{}", port)
}

// Rust's dead code detection seems not to process all integration test crates,
// leading to a false positive on this function.
#[allow(dead_code)]
pub fn gql_endpoint() -> String {
    format!("http://{}/graphql", server_addr())
}

#[allow(dead_code)]
pub fn load_config(config: &str) -> Config {
    let cf = File::open(config).expect("Could not open test model config file.");
    let cr = BufReader::new(cf);
    serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
}

#[allow(dead_code)]
pub fn test_client() -> WarpgrapherClient {
    WarpgrapherClient::new(&gql_endpoint())
}

#[allow(dead_code)]
pub fn clear_db() {
    let graph = GraphClient::connect(db_url()).unwrap();
    graph.exec("MATCH (n) DETACH DELETE (n)").unwrap();
}
