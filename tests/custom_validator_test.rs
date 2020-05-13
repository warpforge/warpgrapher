mod setup;

use log::trace;
use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if the custom validator executes correctly on create mutation
#[tokio::test]
#[serial]
async fn custom_input_validator_create() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // Test validator on create
    // Validator pass
    let result = client
        .create_node("User", "id name", &json!({"name": "ORION"}))
        .await
        .unwrap();

    let name = result.get("name").unwrap();

    assert_eq!(name, "ORION");

    // Validator fail
    let result = client
        .create_node("User", "id name", &json!({"name": "KENOBI"}))
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = match result {
        serde_json::Value::Null => true,
        _ => false,
    };

    assert_eq!(error, true);

    // shutdown server
    assert!(server.shutdown().is_ok());
}

/// Passes if the custom validator executes correctly on update mutation
#[tokio::test]
#[serial]
async fn custom_input_validator_update() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let _ = client
        .create_node("User", "id name", &json!({"name": "ORION"}))
        .await
        .unwrap();

    // Test validator on update
    // Validator pass
    let result = client
        .update_node(
            "User",
            "id name",
            Some(&json!({"name": "ORION"})),
            &json!({"name": "SKYWALKER"}),
        )
        .await
        .unwrap();

    let name = result[0].get("name").unwrap();

    assert_eq!(name, "SKYWALKER");

    // Validator fail
    let result = client
        .update_node(
            "User",
            "id name",
            Some(&json!({"name": "SKYWALKER"})),
            &json!({"name": "KENOBI"}),
        )
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = match result {
        serde_json::Value::Null => true,
        _ => false,
    };

    assert_eq!(error, true);

    // shutdown server
    assert!(server.shutdown().is_ok());
}
