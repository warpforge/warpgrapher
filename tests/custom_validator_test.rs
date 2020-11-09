mod setup;

#[cfg(feature = "neo4j")]
use log::trace;
#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, neo4j_test_client};

/// Passes if the custom validator executes correctly on create mutation
#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_input_validator_create() {
    init();
    clear_db().await;
    let mut client = neo4j_test_client("./tests/fixtures/config.yml").await;

    // Test validator on create
    // Validator pass
    let result = client
        .create_node("User", "id name", Some("1234"), &json!({"name": "ORION"}))
        .await
        .unwrap();

    let name = result.get("name").unwrap();

    assert_eq!(name, "ORION");

    // Validator fail
    let result = client
        .create_node("User", "id name", Some("1234"), &json!({"name": "KENOBI"}))
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = matches!(result, serde_json::Value::Null);

    assert_eq!(error, true);

    // shutdown server
}

/// Passes if the custom validator executes correctly on update mutation
#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_input_validator_update() {
    init();
    clear_db().await;
    let mut client = neo4j_test_client("./tests/fixtures/config.yml").await;

    let _ = client
        .create_node("User", "id name", Some("1234"), &json!({"name": "ORION"}))
        .await
        .unwrap();

    // Test validator on update
    // Validator pass
    let result = client
        .update_node(
            "User",
            "id name",
            Some("1234"),
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
            Some("1234"),
            Some(&json!({"name": "SKYWALKER"})),
            &json!({"name": "KENOBI"}),
        )
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = matches!(result, serde_json::Value::Null);

    assert_eq!(error, true);

    // shutdown server
}
