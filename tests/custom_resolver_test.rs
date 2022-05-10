mod setup;

#[cfg(feature = "cypher")]
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::{clear_db, cypher_test_client, init};

/// Passes if the custom resolvers executes correctly
#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_endpoint_returning_scalar() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
            None,
        )
        .await
        .unwrap();
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "SPARTANII", "description": "Cue MC music"}),
            None,
        )
        .await
        .unwrap();

    // count projects via custom resolver
    let result = client
        .graphql("query { ProjectCount }", None, None, Some("ProjectCount"))
        .await
        .unwrap();

    // verify result
    assert!(result.is_number());
    assert_eq!(result, 2);

    // shutdown server
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_endpoint_returning_scalar_list() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let result = client
        .graphql(
            "
            query { 
                GlobalTopTags 
            }
         ",
            None,
            None,
            Some("GlobalTopTags"),
        )
        .await
        .unwrap();
    assert_eq!(
        result,
        json!(["web", "database", "rust", "python", "graphql"])
    );

    // shutdown server
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_endpoint_returning_node() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let result = client
        .graphql(
            "
            query { 
                GlobalTopDev { 
                    name 
                }
            }
        ",
            None,
            None,
            Some("GlobalTopDev"),
        )
        .await
        .unwrap();
    assert_eq!(result, json!({"name": "Joe"}));

    // shutdown server
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_field_resolver_returning_scalar() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    // create new projects
    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({"name": "ORION", "description": "Intro to supersoldiers"}),
            None,
        )
        .await
        .unwrap();

    let result = client
        .graphql("query { Project{id, points}}", None, None, Some("Project"))
        .await
        .unwrap();
    let points = result[0].get("points").unwrap();

    // verify result
    assert!(points.is_number());
    assert_eq!(*points, json!(138));

    // shutdown server
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_field_returning_scalar_list() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
            None,
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
            None,
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
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_rel_returning_rel() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
            None,
        )
        .await
        .unwrap();

    let _ = client
        .create_node("User", "id name", &json!({"name": "Joe"}), None)
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
            None,
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
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn custom_rel_returning_rel_list() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/config.yml").await;

    let _ = client
        .create_node(
            "Project",
            "id name description",
            &json!({
                "name": "ORION",
                "description": "Intro to supersoldiers"
            }),
            None,
        )
        .await
        .unwrap();

    let _ = client
        .create_node(
            "Feature",
            "id name",
            &json!({ "name" : "Add async support"}),
            None,
        )
        .await
        .unwrap();

    let _ = client
        .create_node(
            "Bug",
            "id name",
            &json!({ "name" : "Fix memory leak" }),
            None,
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
            None,
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
    // shutdown server
}
