mod setup;

use serde_json::json;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use serial_test::serial;
#[cfg(feature = "graphson2")]
use setup::server::test_server_graphson2;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
use setup::test_client;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use setup::{clear_db, init};

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn scalar_lists_test_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_lists_test().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn scalar_lists_test_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_lists_test().await;

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_lists_test() {
    let mut client = test_client();

    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234".to_string()),
            &json!({
                "string_list": ["string0", "string1", "string2", "string3"],
                "bool_list": [true, false, true, false],
                "int_list": [0, 1, 2, 3],
                "float_list": [0.0, 1.1, 2.2, 3.3]
            }),
        )
        .await
        .unwrap();

    let strings = result.get("string_list").unwrap();
    assert!(strings.is_array());
    assert_eq!(strings.get(0).unwrap().as_str().unwrap(), "string0");
    assert_eq!(strings.get(1).unwrap().as_str().unwrap(), "string1");
    assert_eq!(strings.get(2).unwrap().as_str().unwrap(), "string2");
    assert_eq!(strings.get(3).unwrap().as_str().unwrap(), "string3");

    let bools = result.get("bool_list").unwrap();
    assert!(bools.is_array());
    assert_eq!(bools.get(0).unwrap().as_bool().unwrap(), true);
    assert_eq!(bools.get(1).unwrap().as_bool().unwrap(), false);
    assert_eq!(bools.get(2).unwrap().as_bool().unwrap(), true);
    assert_eq!(bools.get(3).unwrap().as_bool().unwrap(), false);

    let ints = result.get("int_list").unwrap();
    assert!(ints.is_array());
    assert_eq!(ints.get(0).unwrap().as_i64().unwrap(), 0);
    assert_eq!(ints.get(1).unwrap().as_i64().unwrap(), 1);
    assert_eq!(ints.get(2).unwrap().as_i64().unwrap(), 2);
    assert_eq!(ints.get(3).unwrap().as_i64().unwrap(), 3);

    let floats = result.get("float_list").unwrap();
    assert!(floats.is_array());
    assert_eq!(floats.get(0).unwrap().as_f64().unwrap(), 0.0_f64);
    assert_eq!(floats.get(1).unwrap().as_f64().unwrap(), 1.1_f64);
    assert_eq!(floats.get(2).unwrap().as_f64().unwrap(), 2.2_f64);
    assert_eq!(floats.get(3).unwrap().as_f64().unwrap(), 3.3_f64);
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn scalar_lists_no_array_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_lists_no_array_test().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn scalar_lists_no_array_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_lists_no_array_test().await;

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_lists_no_array_test() {
    let mut client = test_client();

    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234".to_string()),
            &json!({
                "string_list": "string0",
                "bool_list": false,
                "int_list": 0,
                "float_list": 0.0,
            }),
        )
        .await
        .unwrap();

    let strings = result.get("string_list").unwrap();
    assert!(strings.is_array());
    assert_eq!(strings.get(0).unwrap().as_str().unwrap(), "string0");

    let bools = result.get("bool_list").unwrap();
    assert!(bools.is_array());
    assert_eq!(bools.get(0).unwrap().as_bool().unwrap(), false);

    let ints = result.get("int_list").unwrap();
    assert!(ints.is_array());
    assert_eq!(ints.get(0).unwrap().as_i64().unwrap(), 0);

    let floats = result.get("float_list").unwrap();
    assert!(floats.is_array());
    assert_eq!(floats.get(0).unwrap().as_f64().unwrap(), 0.0_f64);
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn scalar_no_lists_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/scalar_no_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_no_lists_test().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn scalar_no_lists_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/scalar_no_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_no_lists_test().await;

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
async fn scalar_no_lists_test() {
    let mut client = test_client();

    assert!(client
        .create_node(
            "TestType",
            "string_list",
            Some("1234".to_string()),
            &json!({
                "string_list": ["string0", "string1", "string2", "string3"],
            }),
        )
        .await
        .is_err());

    assert!(client
        .create_node(
            "TestType",
            "bool_list",
            Some("1234".to_string()),
            &json!({
                "bool_list": [true, false, true, false],
            }),
        )
        .await
        .is_err());

    assert!(client
        .create_node(
            "TestType",
            "int_list",
            Some("1234".to_string()),
            &json!({
                "int_list": [0, 1, 2, 3],
            }),
        )
        .await
        .is_err());

    assert!(client
        .create_node(
            "TestType",
            "float_list",
            Some("1234".to_string()),
            &json!({
                "float_list": [0.0, 1.1, 2.2, 3.3],
            }),
        ).await
        .is_err());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn scalar_no_lists_no_array_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/scalar_no_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_no_lists_no_array_test().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn scalar_no_lists_no_array_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/scalar_no_list.yml");
    assert!(server.serve(false).is_ok());

    scalar_no_lists_no_array_test().await;

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_no_lists_no_array_test() {
    let mut client = test_client();

    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234".to_string()),
            &json!({
                "string_list": "string0",
                "bool_list": false,
                "int_list": 0,
                "float_list": 0.0,
            }),
        )
        .await
        .unwrap();

    assert_eq!(
        result.get("string_list").unwrap().as_str().unwrap(),
        "string0"
    );

    assert_eq!(result.get("bool_list").unwrap().as_bool().unwrap(), false);

    assert_eq!(result.get("int_list").unwrap().as_i64().unwrap(), 0);

    assert_eq!(result.get("float_list").unwrap().as_f64().unwrap(), 0.0_f64);
}
