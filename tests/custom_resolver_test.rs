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
async fn custom_endpoint_returning_scalar() {
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
            Some("1234"),
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();
    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234"),
            &json!({"name": "SPARTANII", "description": "Cue MC music"}),
        )
        .await
        .unwrap();

    // count projects via custom resolver
    let result = client
        .graphql("query { ProjectCount }", Some("1234"), None, "ProjectCount")
        .await
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
async fn custom_endpoint_returning_scalar_list() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let result = client
        .graphql(
            "
            query { 
                GlobalTopTags 
            }
         ",
            Some("1234"),
            None,
            "GlobalTopTags",
        )
        .await
        .unwrap();
    let tags = result.get("GlobalTopTags").unwrap();
    assert_eq!(
        *tags,
        json!(["web", "database", "rust", "python", "graphql"])
    );

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_endpoint_returning_node() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let result = client
        .graphql(
            "
            query { 
                GlobalTopDev { 
                    name 
                }
            }
        ",
            Some("1234"),
            None,
            "GlobalTopDev",
        )
        .await
        .unwrap();
    let topdev = result.get("GlobalTopDev").unwrap();
    assert_eq!(*topdev, json!({"name": "Joe"}));

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_field_resolver_returning_scalar() {
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
            Some("1234"),
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
        )
        .await
        .unwrap();

    let result = client
        .graphql(
            "query { Project{id, points}}",
            Some("1234"),
            None,
            "Project",
        )
        .await
        .unwrap();
    let project = result.get("Project").unwrap();
    let points = project[0].get("points").unwrap();

    // verify result
    assert!(points.is_number());
    assert_eq!(*points, json!(138));

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_field_returning_scalar_list() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234"),
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            toptags",
            Some("1234"),
            None,
        )
        .await
        .unwrap();
    assert!(projects.is_array());
    let p0 = projects.as_array().unwrap().get(0).unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(
        *p0.get("toptags").unwrap(),
        json!(["cypher", "sql", "neo4j"])
    );

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_rel_returning_rel() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234"),
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            topdev {
                dst {
                    ... on User {
                        name
                    }
                }
            }
            ",
            Some("1234"),
            None,
        )
        .await
        .unwrap();
    assert!(projects.is_array());
    let p0 = projects.as_array().unwrap().get(0).unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    let p0_topdevs = p0.get("topdev").unwrap();
    let p0_topdevs_dst = p0_topdevs.get("dst").unwrap();
    assert_eq!(*p0_topdevs_dst, json!({"name": "Joe"}));

    // shutdown server
    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn custom_rel_returning_rel_list() {
    init();
    clear_db();
    let mut client = neo4j_test_client();
    let mut server = test_server_neo4j("./tests/fixtures/config.yml");
    assert!(server.serve(false).is_ok());

    let _ = client
        .create_node(
            "Project",
            "id name description",
            Some("1234"),
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            topissues {
                dst {
                    ... on Feature {
                        name
                    }
                    ... on Bug {
                        name
                    }
                }
            }
            ",
            Some("1234"),
            None,
        )
        .await
        .unwrap();
    assert!(projects.is_array());
    let p0 = projects.as_array().unwrap().get(0).unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    let p0_topissues = p0.get("topissues").unwrap().as_array().unwrap();
    assert_eq!(p0_topissues.len(), 2);
    let i0 = p0_topissues.get(0).unwrap();
    assert_eq!(*i0, json!({"dst": {"name": "Add async support"}}));
    let i1 = p0_topissues.get(1).unwrap();
    assert_eq!(*i1, json!({"dst": {"name": "Fix type mismatch"}}));

    // shutdown server
    assert!(server.shutdown().is_ok());
}
