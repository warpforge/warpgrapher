mod setup;

#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use serial_test::serial;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(feature = "neo4j")]
use setup::{clear_db, gql_endpoint, init, test_client};
#[cfg(feature = "neo4j")]
use warpgrapher::client::graphql;

/// Passes if the custom resolvers executes correctly
#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn custom_endpoint_resolver() {
    init();
    clear_db();
    let mut client = test_client();
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
        .unwrap();
    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234".to_string()),
            &json!({"name": "SPARTANII", "description": "Cue MC music"}),
        )
        .unwrap();

    // count projects via custom resolver
    let result = graphql(
        gql_endpoint(),
        "query { ProjectCount }".to_owned(),
        Some("1234".to_string()),
        None,
    )
    .unwrap();
    let count = result.get("ProjectCount").unwrap();

    // verify result
    assert!(count.is_number());
    assert_eq!(count, 2);

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn custom_prop_resolver() {
    init();
    clear_db();
    let mut client = test_client();
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
        .unwrap();

    let result = graphql(
        gql_endpoint(),
        "query { Project{id, points}}".to_owned(),
        Some("1234".to_string()),
        None,
    )
    .unwrap();
    let project = result.get("Project").unwrap();
    let points = project[0].get("points").unwrap();

    // verify result
    assert!(points.is_number());
    assert_eq!(*points, json!(1_000_000));

    // shutdown server
    assert!(server.shutdown().is_ok());
}
