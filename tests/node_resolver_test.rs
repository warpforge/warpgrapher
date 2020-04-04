mod setup;

use assert_approx_eq::assert_approx_eq;
#[cfg(feature = "neo4j")]
use rusted_cypher::GraphClient;
use serde_json::json;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use serial_test::serial;
#[cfg(feature = "neo4j")]
use setup::neo4j_url;
#[cfg(feature = "graphson2")]
use setup::server::test_server_graphson2;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
use setup::test_client;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use setup::{clear_db, init};

/// Passes if the create mutation and the read query both succeed.
#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn create_single_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_single_node();

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn create_single_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_single_node();

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
fn create_single_node() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active", Some("1234".to_string()),
            &json!({"name": "MJOLNIR", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
        )
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
            Some("1234".to_string()),
            None,
        )
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
#[serial(neo4j)]
#[test]
fn read_query_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_query();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn ready_query_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_query();

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[allow(dead_code)]
fn read_query() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234".to_string()),
            &json!({"name": "Project1"}),
        )
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234".to_string()),
            &json!({"name": "Project2"}),
        )
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "Project1");
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn handle_missing_properties_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    handle_missing_properties();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn handle_missing_properties_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    handle_missing_properties();

    assert!(server.shutdown().is_ok());
}

/// Passes if resolvers can handle a shape that reads a property that is not
/// present on the Neo4J model object.
#[allow(dead_code)]
fn handle_missing_properties() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name description",
            Some("1234".to_string()),
            &json!({"name": "MJOLNIR"}),
        )
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert!(p0.get("description").unwrap().is_null());

    let projects = client
        .read_node(
            "Project",
            "__typename id name description",
            Some("1234".to_string()),
            None,
        )
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
#[serial(neo4j)]
#[test]
fn update_mutation_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mutation();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mutation_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mutation();

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a target node selected by attribute
#[allow(dead_code)]
fn update_mutation() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
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
            Some("1234".to_string()),
            Some(&json!({"name": "Project1"})),
            &json!({"status": "ACTIVE"}),
        )
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
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
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
#[serial(neo4j)]
#[test]
fn update_mutation_null_query_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mutation_null_query();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mutation_null_query_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mutation_null_query();

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a null match, meaning update all nodes
#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mutation_null_query() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            None,
        )
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            None,
            &json!({"status": "ACTIVE"}),
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(pu_a[1].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[1].get("status").unwrap(), "ACTIVE");
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            None,
        )
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 2);
    assert_eq!(after_projects_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(after_projects_a[1].get("status").unwrap(), "ACTIVE");
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_mutation_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mutation();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mutation_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mutation();

    assert!(server.shutdown().is_ok());
}

/// Passes if the delete mutation succeeds with a target node selected by attribute
#[allow(dead_code)]
fn delete_mutation() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
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
            Some("1234".to_string()),
            Some(&json!({"name": "Project1"})),
            None,
        )
        .unwrap();

    assert_eq!(pd, 1);
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_mutation_null_query_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mutation_null_query();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mutation_null_query_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mutation_null_query();

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a null match, meaning delete all nodes
#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mutation_null_query() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project1", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project1");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            None,
        )
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pd = client
        .delete_node("Project", Some("1234".to_string()), None, None)
        .unwrap();

    assert_eq!(pd, 2);

    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            Some(json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn error_on_node_missing_id_neo4j() {
    init();
    clear_db();

    let graph = GraphClient::connect(neo4j_url()).unwrap();
    graph
        .exec("CREATE (n:Project { name: 'Project One' })")
        .unwrap();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    error_on_node_missing_id();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn error_on_node_missing_id_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    error_on_node_missing_id();

    assert!(server.shutdown().is_ok());
}

/// Passes if creating a node manually without an id throws an error upon access
/// to that node.
#[allow(dead_code)]
fn error_on_node_missing_id() {
    let mut client = test_client();

    let projects = client
        .read_node(
            "Project",
            "__typename id name",
            Some("1234".to_string()),
            Some(json!({"name": "Project One"})),
        )
        .unwrap();

    assert!(projects.is_null());

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
            &json!({"status": "ACTIVE"}),
        )
        .unwrap();

    assert!(pu.is_null());

    let pd = client
        .delete_node(
            "Project",
            Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
            None,
        )
        .unwrap();

    assert!(pd.is_null());
}
