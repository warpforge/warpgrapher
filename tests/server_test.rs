mod setup;

#[cfg(feature = "neo4j")]
use serial_test::serial;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(feature = "neo4j")]
use setup::{init, load_config};
#[cfg(feature = "neo4j")]
use warpgrapher::server::database::DatabaseEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::Neo4jEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::{ErrorKind, Server};

/// Passes if the server can be created, run, and shut down.
#[cfg(feature = "neo4j")]
#[test]
#[serial(neo4j)]
fn server_new_serve_shutdown() {
    init();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[test]
#[serial(neo4j)]
fn server_endpoints_new_serve_shutdown() {
    init();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());
    assert!(server.shutdown().is_ok());
}

/// Passes if the server will not run if already running
#[cfg(feature = "neo4j")]
#[test]
#[serial(neo4j)]
fn server_already_serving() {
    init();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let e = server.serve(false).expect_err("Server started twice");
    assert!(match e.kind {
        ErrorKind::ServerAlreadyRunning => true,
        _ => false,
    });

    assert!(server.shutdown().is_ok());
}

/// Passes if the server will not bind to a nonsensical address
#[cfg(feature = "neo4j")]
#[test]
#[serial(neo4j)]
fn server_bad_address() {
    init();

    let config = load_config("tests/fixtures/test_config_ok.yml");
    let mut server = Server::<(), ()>::new(
        config,
        Neo4jEndpoint::from_env()
            .expect("Expected endpoint from env vars.")
            .get_pool()
            .expect("Expected database pool."),
    )
    .with_bind_addr("1.2.3.4:5".to_owned())
    .build()
    .unwrap();

    let e = server
        .serve(false)
        .expect_err("Server started with bad address");

    assert!(match e.kind {
        ErrorKind::AddrNotAvailable(..) => true,
        _ => false,
    });
}
