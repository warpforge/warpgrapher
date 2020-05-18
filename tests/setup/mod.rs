#[cfg(any(feature = "cosmos", feature = "neo4j"))]
mod actix_server;
mod extension;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(crate) mod server;

#[cfg(feature = "cosmos")]
use gremlin_client::{ConnectionOptions, GraphSON, GremlinClient};
#[cfg(feature = "cosmos")]
use log::trace;
#[cfg(feature = "neo4j")]
use rusted_cypher::GraphClient;
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
use warpgrapher::client::Client;
use warpgrapher::engine::config::Config;

#[allow(dead_code)]
pub(crate) fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[cfg(feature = "cosmos")]
fn cosmos_host() -> String {
    var_os("WG_COSMOS_HOST")
        .expect("Expected WG_COSMOS_HOST to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_HOST to be a string.")
        .to_owned()
}

#[cfg(feature = "cosmos")]
fn cosmos_port() -> u16 {
    var_os("WG_COSMOS_PORT")
        .expect("Expected WG_COSMOS_PORT to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_PORT to be a string.")
        .parse::<u16>()
        .expect("Expected WG_COSMOS_PORT to be a u16.")
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_login() -> String {
    var_os("WG_COSMOS_LOGIN")
        .expect("Expected WG_COSMOS_LOGIN to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_LOGIN to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_pass() -> String {
    var_os("WG_COSMOS_PASS")
        .expect("Expected WG_COSMOS_PASS to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_PASS to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_url() -> String {
    var_os("WG_NEO4J_URL")
        .expect("Expected WG_NEO4J_URL to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_URL to be a string.")
        .to_owned()
}

#[allow(dead_code)]
fn server_addr() -> String {
    let port = match var_os("WG_SAMPLE_PORT") {
        None => 5000,
        Some(os) => os.to_str().unwrap_or("5000").parse::<u16>().unwrap_or(5000),
    };

    format!("127.0.0.1:{}", port)
}

fn neo4j_server_addr() -> String {
    "127.0.0.1:5000".to_string()
}

fn cosmos_server_addr() -> String {
    "127.0.0.1:5001".to_string()
}

// Rust's dead code detection seems not to process all integration test crates,
// leading to a false positive on this function.
#[allow(dead_code)]
fn gql_endpoint() -> String {
    format!("http://{}/graphql", server_addr())
}

fn neo4j_gql_endpoint() -> String {
    format!("http://{}/graphql", neo4j_server_addr())
}

fn cosmos_gql_endpoint() -> String {
    format!("http://{}/graphql", cosmos_server_addr())
}

#[allow(dead_code)]
fn load_config(config: &str) -> Config {
    let cf = File::open(config).expect("Could not open test model config file.");
    let cr = BufReader::new(cf);
    serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
}

#[allow(dead_code)]
pub(crate) fn neo4j_test_client() -> Client {
    Client::new(&neo4j_gql_endpoint())
}

#[allow(dead_code)]
pub(crate) fn cosmos_test_client() -> Client {
    Client::new(&cosmos_gql_endpoint())
}

#[cfg(feature = "cosmos")]
#[allow(dead_code)]
fn clear_cosmos_db() {
    // g.V().drop() -- delete the entire graph
    let client = GremlinClient::connect(
        ConnectionOptions::builder()
            .host(cosmos_host())
            .port(cosmos_port())
            .pool_size(1)
            .ssl(true)
            .serializer(GraphSON::V2)
            .credentials(&cosmos_login(), &cosmos_pass())
            .build(),
    )
    .expect("Expected successful gremlin client creation.");

    let results = client.execute("g.V().drop()", &[]);

    trace!("{:#?}", results);
}

#[cfg(feature = "neo4j")]
#[allow(dead_code)]
fn clear_neo4j_db() {
    let graph = GraphClient::connect(neo4j_url()).unwrap();
    graph.exec("MATCH (n) DETACH DELETE (n)").unwrap();
}

#[allow(dead_code)]
pub(crate) fn clear_db() {
    #[cfg(feature = "cosmos")]
    clear_cosmos_db();

    #[cfg(feature = "neo4j")]
    clear_neo4j_db();
}
