mod setup;

#[cfg(feature = "cypher")]
use log::trace;
#[cfg(feature = "cypher")]
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::{clear_db, cypher_test_client, init};

/// Passes if the custom validator executes correctly on create mutation
#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_input_validator_create() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    // Test validator on create
    // Validator pass
    let result = client
        .create_node("User", "id name", &json!({"name": "ORION"}), None)
        .await
        .unwrap();

    let name = result.get("name").unwrap();

    assert_eq!(name, "ORION");

    // Validator fail
    let result = client
        .create_node("User", "id name", &json!({"name": "KENOBI"}), None)
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = matches!(result, serde_json::Value::Null);

    assert!(error);

    // shutdown server
}

/// Passes if the custom validator executes correctly on update mutation
#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_input_validator_update() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let _ = client
        .create_node("User", "id name", &json!({"name": "ORION"}), None)
        .await
        .unwrap();

    // Test validator on update
    // Validator pass
    let result = client
        .update_node(
            "User",
            "id name",
            Some(&json!({"name": {"EQ": "ORION"}})),
            &json!({"name": "SKYWALKER"}),
            None,
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
            Some(&json!({"name": {"EQ": "SKYWALKER"}})),
            &json!({"name": "KENOBI"}),
            None,
        )
        .await
        .unwrap();

    trace!("RESULT: {:#?}", result);
    let error = matches!(result, serde_json::Value::Null);

    assert!(error);

    // shutdown server
}
