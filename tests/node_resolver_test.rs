mod setup;

use assert_approx_eq::assert_approx_eq;
use rusted_cypher::GraphClient;
use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, db_url, init, test_client};

/// Passes if the create mutation and the read query both succeed.
#[test]
#[serial]
fn create_single_node() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active",
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

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[test]
#[serial]
fn read_query() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
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
            Some(&json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "Project1");

    assert!(server.shutdown().is_ok());
}

/// Passes if resolvers can handle a shape that reads a property that is not
/// present on the Neo4J model object.
#[test]
#[serial]
fn handle_missing_properties() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name description",
            &json!({"name": "MJOLNIR"}),
        )
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert!(p0.get("description").unwrap().is_null());

    let projects = client
        .read_node("Project", "__typename id name description", None)
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(projects_a[0].get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(projects_a[0].get("name").unwrap(), "MJOLNIR");
    assert!(projects_a[0].get("description").unwrap().is_null());

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a target node selected by attribute
#[test]
#[serial]
fn update_mutation() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
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
            Some(&json!({"name": "Project1"})),
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
            Some(&json!({"name": "Project1"})),
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

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a null match, meaning update all nodes
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mutation_null_query() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
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
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", None)
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
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
        .read_node("Project", "__typename id name status", None)
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 2);
    assert_eq!(after_projects_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(after_projects_a[1].get("status").unwrap(), "ACTIVE");

    assert!(server.shutdown().is_ok());
}

/// Passes if the delete mutation succeeds with a target node selected by attribute
#[test]
#[serial]
fn delete_mutation() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
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
            Some(&json!({"name": "Project1"})),
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
        .delete_node("Project", Some(&json!({"name": "Project1"})), None)
        .unwrap();

    assert_eq!(pd, 1);
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);

    assert!(server.shutdown().is_ok());
}

/// Passes if the update mutation succeeds with a null match, meaning delete all nodes
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mutation_null_query() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
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
            &json!({"name": "Project2", "status": "PENDING"}),
        )
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", None)
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pd = client.delete_node("Project", None, None).unwrap();

    assert_eq!(pd, 2);

    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": "Project1"})),
        )
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);

    assert!(server.shutdown().is_ok());
}

/// Passes if creating a node manually without an id throws an error upon access
/// to that node.
#[test]
#[serial]
fn error_on_node_missing_id() {
    init();
    clear_db();

    let graph = GraphClient::connect(db_url()).unwrap();
    graph
        .exec("CREATE (n:Project { name: 'Project One' })")
        .unwrap();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let projects = client
        .read_node(
            "Project",
            "__typename id name",
            Some(&json!({"name": "Project One"})),
        )
        .unwrap();

    assert!(projects.is_null());

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": "Project One"})),
            &json!({"status": "ACTIVE"}),
        )
        .unwrap();

    assert!(pu.is_null());

    let pd = client
        .delete_node("Project", Some(&json!({"name": "Project One"})), None)
        .unwrap();

    assert!(pd.is_null());

    assert!(server.shutdown().is_ok());
}
