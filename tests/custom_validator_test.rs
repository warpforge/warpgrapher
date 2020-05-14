mod setup;

#[cfg(feature = "neo4j")]
use log::trace;
#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use serial_test::serial;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, test_client};

/// Passes if the custom validator executes correctly on create mutation
#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn custom_input_validator_create() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // Test validator on create
    // Validator pass
    let result = client
        .create_node(
            "User",
            "id name",
            Some("1234".to_string()),
            &json!({"name": "ORION"}),
        ).await
        .unwrap();

    let name = result.get("name").unwrap();

    assert_eq!(name, "ORION");

    // Validator fail
    let result = client
        .create_node(
            "User",
            "id name",
            Some("1234".to_string()),
            &json!({"name": "KENOBI"}),
        ).await
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
#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn custom_input_validator_update() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let _ = client
        .create_node(
            "User",
            "id name",
            Some("1234".to_string()),
            &json!({"name": "ORION"}),
        ).await
        .unwrap();

    // Test validator on update
    // Validator pass
    let result = client
        .update_node(
            "User",
            "id name",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
