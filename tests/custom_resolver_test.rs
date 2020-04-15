mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if the custom resolvers executes correctly
#[tokio::test]
#[serial]
async fn custom_endpoint_resolver() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "SPARTANII", "description": "Cue MC music"}),
        )
        .await
        .unwrap();

    // count projects via custom resolver
    let result = client.graphql("query { ProjectCount }", None).await.unwrap();
    let count = result.get("ProjectCount").unwrap();

    // verify result
    assert!(count.is_number());
    assert_eq!(count, 2);

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[tokio::test]
#[serial]
async fn custom_prop_resolver() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();

    let result = client.graphql(
        "query { Project{id, points}}",
        None,
    )
    .await
    .unwrap();
    let project = result.get("Project").unwrap();
    let points = project[0].get("points").unwrap();

    // verify result
    assert!(points.is_number());
    assert_eq!(*points, json!(1_000_000));

    // shutdown server
    assert!(server.shutdown().is_ok());
}
