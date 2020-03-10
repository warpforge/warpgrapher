mod setup;

use serial_test::serial;
use setup::server::test_server;
//use setup::{db_url, init, load_config};
use setup::init;
use warpgrapher::ErrorKind;
//use warpgrapher::Server;

/// Passes if the server can be created, run, and shut down.
#[test]
#[serial]
fn server_new_serve_shutdown() {
    init();

    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());
    assert!(server.shutdown().is_ok());
}

#[test]
#[serial]
fn server_endpoints_new_serve_shutdown() {
    init();

    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());
    assert!(server.shutdown().is_ok());
}

/// Passes if the server will not run if already running
#[test]
#[serial]
fn server_already_serving() {
    init();

    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let e = server.serve(false).expect_err("Server started twice");
    assert!(match e.kind {
        ErrorKind::ServerAlreadyRunning => true,
        _ => false,
    });

    assert!(server.shutdown().is_ok());
}

/*
/// Passes if the server will not bind to a nonsensical address
#[test]
#[serial]
#[ignore]
fn server_bad_address() {
    init();

    let config = load_config("tests/fixtures/test_config_ok.yml");
    let mut server = Server::<()>::new(config, db_url())
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
*/

/*
/// Passes if the server cannot bind to a port already being used
#[test]
#[serial]
#[ignore]
fn server_duplicate_port() {
    init();

    let mut server1 = test_server("./tests/fixtures/minimal.yml");
    assert!(server1.serve(false).is_ok());

    let mut server2 = test_server("./tests/fixtures/minimal.yml");
    let e = server2.serve(false).expect_err("Bound to duplicate port");
    assert!(match e.kind {
        ErrorKind::AddrInUse(..) => true,
        _ => false,
    });

    assert!(server1.shutdown().is_ok());
}
*/
