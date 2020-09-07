mod setup;

use assert_approx_eq::assert_approx_eq;
use serde_json::json;
#[cfg(feature = "neo4j")]
use setup::bolt_client;
#[cfg(feature = "cosmos")]
use setup::cosmos_test_client;
#[cfg(feature = "gremlin")]
use setup::gremlin_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use setup::{clear_db, init};
use setup::{AppGlobalCtx, AppRequestCtx};
#[cfg(feature = "neo4j")]
use std::iter::FromIterator;
use warpgrapher::client::Client;

/// Passes if the create mutation and the read query both succeed.
#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_single_node_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_single_node(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_single_node_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_single_node(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_single_node_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    create_single_node(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
async fn create_single_node(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active", Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
            // &json!({"name": "MJOLNIR", "description": "Powered armor", "status": "GREEN"}),
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Powered armor");
    assert_eq!(p0.get("status").unwrap(), "GREEN");
    assert_eq!(p0.get("priority").unwrap(), 1);
    assert_approx_eq!(p0.get("estimate").unwrap().as_f64().unwrap(), 3.3);
    assert_eq!(p0.get("active").unwrap(), true);

    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "MJOLNIR");
    assert_eq!(projects_a[0].get("description").unwrap(), "Powered armor");
    assert_eq!(projects_a[0].get("status").unwrap(), "GREEN");
    assert_eq!(projects_a[0].get("priority").unwrap(), 1);
    assert_approx_eq!(
        projects_a[0].get("estimate").unwrap().as_f64().unwrap(),
        3.3
    );
    assert_eq!(projects_a[0].get("active").unwrap(), true);
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_query_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_query(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_query_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_query(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_query_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_query(client).await;
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
async fn read_query(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234"),
            &json!({"name": "Project1"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234"),
            &json!({"name": "Project2"}),
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "Project1");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn handle_missing_properties_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    handle_missing_properties(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn handle_missing_properties_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    handle_missing_properties(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn handle_missing_properties_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    handle_missing_properties(client).await;
}

/// Passes if resolvers can handle a shape that reads a property that is not
/// present on the Neo4J model object.
#[allow(dead_code)]
async fn handle_missing_properties(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description",
            Some("1234"),
            &json!({"name": "MJOLNIR"}),
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert!(p0.get("description").unwrap().is_null());

    let projects = client
        .read_node(
            "Project",
            "__typename id name description",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "MJOLNIR");
    assert!(projects_a[0].get("description").unwrap().is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mutation_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mutation_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_mutation_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation(client).await;
}

/// Passes if the update mutation succeeds with a target node selected by attribute
#[allow(dead_code)]
async fn update_mutation(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 1);
    assert_eq!(before_projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(
        before_projects_a[0].get("id").unwrap(),
        p0.get("id").unwrap()
    );
    assert_eq!(before_projects_a[0].get("name").unwrap(), "Project1");
    assert_eq!(before_projects_a[0].get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
            &json!({"status": "ACTIVE"}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "Project1");
    assert_eq!(pu_a[0].get("status").unwrap(), "ACTIVE");
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 1);
    assert_eq!(after_projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(
        after_projects_a[0].get("id").unwrap(),
        p0.get("id").unwrap()
    );
    assert_eq!(after_projects_a[0].get("name").unwrap(), "Project1");
    assert_eq!(after_projects_a[0].get("status").unwrap(), "ACTIVE");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mutation_null_query_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation_null_query(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mutation_null_query_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation_null_query(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_mutation_null_query_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_mutation_null_query(client).await;
}

/// Passes if the update mutation succeeds with a null match, meaning update all nodes
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mutation_null_query(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", Some("1234"), None)
        .await
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            None,
            &json!({"status": "ACTIVE"}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(pu_a[1].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[1].get("status").unwrap(), "ACTIVE");
    let after_projects = client
        .read_node("Project", "__typename id name status", Some("1234"), None)
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 2);
    assert_eq!(after_projects_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(after_projects_a[1].get("status").unwrap(), "ACTIVE");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mutation_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mutation_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_mutation_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation(client).await;
}

/// Passes if the delete mutation succeeds with a target node selected by attribute
#[allow(dead_code)]
async fn delete_mutation(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 1);
    assert_eq!(before_projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(
        before_projects_a[0].get("id").unwrap(),
        p0.get("id").unwrap()
    );
    assert_eq!(before_projects_a[0].get("name").unwrap(), "Project1");
    assert_eq!(before_projects_a[0].get("status").unwrap(), "PENDING");

    let pd = client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
            None,
        )
        .await
        .unwrap();

    assert_eq!(pd, 1);
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mutation_null_query_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation_null_query(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mutation_null_query_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation_null_query(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_mutation_null_query_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_mutation_null_query(client).await;
}

/// Passes if the update mutation succeeds with a null match, meaning delete all nodes
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mutation_null_query(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", Some("1234"), None)
        .await
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pd = client
        .delete_node("Project", Some("1234"), None, None)
        .await
        .unwrap();

    assert_eq!(pd, 2);

    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project1"})),
        )
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn error_on_node_missing_id_neo4j() {
    init();
    clear_db().await;

    let mut graph = bolt_client().await;
    graph
        .run_with_metadata("CREATE (n:Project { name: 'Project One' })", None, None)
        .await
        .expect("Expected successful query run.");

    let pull_meta = bolt_client::Metadata::from_iter(vec![("n", 1)]);
    let (_response, _records) = graph
        .pull(Some(pull_meta))
        .await
        .expect("Expected pull to succeed.");

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    error_on_node_missing_id(client).await;
}

/// Passes if creating a node manually without an id throws an error upon access
/// to that node.  There is no GraphSON variant of this test, because GraphSON
/// data stores automatically assign a UUID id.
#[allow(dead_code)]
async fn error_on_node_missing_id(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let projects = client
        .read_node(
            "Project",
            "__typename id name",
            Some("1234"),
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    assert!(projects.is_null());

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some("1234"),
            Some(&json!({"name": "Project One"})),
            &json!({"status": "ACTIVE"}),
        )
        .await
        .unwrap();

    assert!(pu.is_null());

    // No error thrown for deletion of nodes, because the node ID isn't accessed during deletion.
    // Therefore, there is no delete_node test here.
}
