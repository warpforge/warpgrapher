mod setup;

#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, neo4j_test_client};

/// Passes if the custom resolvers executes correctly
#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_endpoint_resolver() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234".to_string()),
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();
    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234".to_string()),
            &json!({"name": "SPARTANII", "description": "Cue MC music"}),
        )
        .await
        .unwrap();

    // count projects via custom resolver
    let result = client.graphql(
        "query { ProjectCount }",
        Some("1234".to_string()),
        None,
    ).await
    .unwrap();
    let count = result.get("ProjectCount").unwrap();

    // verify result
    assert!(count.is_number());
    assert_eq!(count, 2);

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_prop_resolver() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234".to_string()),
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();

    let result = client.graphql(
        "query { Project{id, points}}",
        Some("1234".to_string()),
        None,
    ).await
    .unwrap();
    let project = result.get("Project").unwrap();
    let points = project[0].get("points").unwrap();

    // verify result
    assert!(points.is_number());
    assert_eq!(*points, json!(1_000_000));

    // shutdown server
    assert!(server.shutdown().is_ok());
}
