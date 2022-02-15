mod setup;

use assert_approx_eq::assert_approx_eq;
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::cypher_test_client;
#[cfg(feature = "gremlin")]
use setup::gremlin_test_client;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
use setup::{clear_db, init};
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn scalar_lists_test_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/scalars/scalar_list.yml").await;
    scalar_lists_test(client).await;
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn scalar_lists_test_cypher() {
    init();
    clear_db().await;

    let client = cypher_test_client("./tests/fixtures/scalars/scalar_list.yml").await;
    scalar_lists_test(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_lists_test<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234"),
            &json!({
                "string_list": ["string0", "string1", "string2", "string3"],
                "bool_list": [true, false],
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
    assert!(bools.get(0).unwrap().as_bool().unwrap());
    assert!(!bools.get(1).unwrap().as_bool().unwrap());

    let ints = result.get("int_list").unwrap();
    assert!(ints.is_array());
    assert_eq!(ints.get(0).unwrap().as_i64().unwrap(), 0);
    assert_eq!(ints.get(1).unwrap().as_i64().unwrap(), 1);
    assert_eq!(ints.get(2).unwrap().as_i64().unwrap(), 2);
    assert_eq!(ints.get(3).unwrap().as_i64().unwrap(), 3);

    let floats = result.get("float_list").unwrap();
    assert!(floats.is_array());
    assert_approx_eq!(floats.get(0).unwrap().as_f64().unwrap(), 0.0_f64);
    assert_approx_eq!(floats.get(1).unwrap().as_f64().unwrap(), 1.1_f64);
    assert_approx_eq!(floats.get(2).unwrap().as_f64().unwrap(), 2.2_f64);
    assert_approx_eq!(floats.get(3).unwrap().as_f64().unwrap(), 3.3_f64);
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn scalar_lists_no_array_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/scalars/scalar_list.yml").await;
    scalar_lists_no_array_test(client).await;
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn scalar_lists_no_array_cypher() {
    init();
    clear_db().await;

    let client = cypher_test_client("./tests/fixtures/scalars/scalar_list.yml").await;
    scalar_lists_no_array_test(client).await;
}

/// Passes if the create mutation and the read query both succeed. Note, if you want an
/// array/list on a node, and you're starting with a single element, you still have to send
/// and array of one element. Sending a single element not wrapped as a list will and should
/// fail, as it sets a scalar property value rather than a list and is returned as such.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_lists_no_array_test<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234"),
            &json!({
                "string_list": ["string0"],
                "bool_list": [false],
                "int_list": [0],
                "float_list": [0.0],
            }),
        )
        .await
        .unwrap();

    let strings = result.get("string_list").unwrap();
    assert!(strings.is_array());
    assert_eq!(strings.get(0).unwrap().as_str().unwrap(), "string0");

    let bools = result.get("bool_list").unwrap();
    assert!(bools.is_array());
    assert!(!bools.get(0).unwrap().as_bool().unwrap());

    let ints = result.get("int_list").unwrap();
    assert!(ints.is_array());
    assert_eq!(ints.get(0).unwrap().as_i64().unwrap(), 0);

    let floats = result.get("float_list").unwrap();
    assert!(floats.is_array());
    assert_eq!(floats.get(0).unwrap().as_f64().unwrap(), 0.0_f64);
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn scalar_no_lists_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/scalars/scalar_no_list.yml").await;
    scalar_no_lists_test(client).await;
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn scalar_no_lists_cypher() {
    init();
    clear_db().await;

    let client = cypher_test_client("./tests/fixtures/scalars/scalar_no_list.yml").await;
    scalar_no_lists_test(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
async fn scalar_no_lists_test<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    assert!(client
        .create_node(
            "TestType",
            "string_list",
            Some("1234"),
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
            Some("1234"),
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
            Some("1234"),
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
            Some("1234"),
            &json!({
                "float_list": [0.0, 1.1, 2.2, 3.3],
            }),
        )
        .await
        .is_err());
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn scalar_no_lists_no_array_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/scalars/scalar_no_list.yml").await;
    scalar_no_lists_no_array_test(client).await;
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn scalar_no_lists_no_array_cypher() {
    init();
    clear_db().await;

    let client = cypher_test_client("./tests/fixtures/scalars/scalar_no_list.yml").await;
    scalar_no_lists_no_array_test(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[allow(clippy::float_cmp, dead_code)]
async fn scalar_no_lists_no_array_test<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            Some("1234"),
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

    assert!(!result.get("bool_list").unwrap().as_bool().unwrap());

    assert_eq!(result.get("int_list").unwrap().as_i64().unwrap(), 0);

    assert_eq!(result.get("float_list").unwrap().as_f64().unwrap(), 0.0_f64);
}
