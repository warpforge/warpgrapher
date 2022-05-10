mod setup;

use assert_approx_eq::assert_approx_eq;
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::{bolt_transaction, clear_db, cypher_test_client, init, CypherRequestCtx};
#[cfg(feature = "cypher")]
use std::collections::HashMap;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::Transaction;
use warpgrapher_macros::wg_test;

/// Passes if the create mutation and the read query both succeed.
#[wg_test]
#[allow(dead_code)]
async fn create_single_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active",
            &json!({"name": "MJOLNIR", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
            None
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
    assert!(p0.get("active").unwrap().as_bool().unwrap());

    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            None,
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
    assert!(projects_a[0].get("active").unwrap().as_bool().unwrap());
}

/// Passes if the create mutation and the read query both succeed, with a specified id.
#[wg_test]
#[allow(dead_code)]
async fn create_single_node_with_id<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active",
            &json!({"id": "12345", "name": "MJOLNIR", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
            None
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("id").unwrap(), "12345");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Powered armor");
    assert_eq!(p0.get("status").unwrap(), "GREEN");
    assert_eq!(p0.get("priority").unwrap(), 1);
    assert_approx_eq!(p0.get("estimate").unwrap().as_f64().unwrap(), 3.3);
    assert!(p0.get("active").unwrap().as_bool().unwrap());

    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some(&json!({"id": {"EQ": "12345"}})),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("id").unwrap(), "12345");
    assert_eq!(projects_a[0].get("name").unwrap(), "MJOLNIR");
    assert_eq!(projects_a[0].get("description").unwrap(), "Powered armor");
    assert_eq!(projects_a[0].get("status").unwrap(), "GREEN");
    assert_eq!(projects_a[0].get("priority").unwrap(), 1);
    assert_approx_eq!(
        projects_a[0].get("estimate").unwrap().as_f64().unwrap(),
        3.3
    );
    assert!(projects_a[0].get("active").unwrap().as_bool().unwrap());
}

/// Passes if the create mutation and the read query both succeed.
#[wg_test]
#[allow(dead_code)]
async fn read_query<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project1"}),
            None,
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
            &json!({"name": "Project2"}),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
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

/// Passes if reading a non-existent node returns an empty array rather than an error
#[wg_test]
#[allow(dead_code)]
async fn read_with_no_result<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let projects = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some(&json!({"id": {"EQ": "1234"}})),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert!(projects_a.is_empty());
}

/// Passes if reading nodes with specific ordering works
#[wg_test]
#[allow(dead_code)]
async fn read_by_order<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name priority",
            &json!({"name": "Project Bravo", "priority": 2}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Bravo");
    assert_eq!(p0.get("priority").unwrap(), 2);

    let p1 = client
        .create_node(
            "Project",
            "__typename id name priority",
            &json!({"name": "Project Alpha", "priority": 3}),
            None,
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Alpha");
    assert_eq!(p1.get("priority").unwrap(), 3);

    let p2 = client
        .create_node(
            "Project",
            "__typename id name priority",
            &json!({"name": "Project Charlie", "priority": 1}),
            None,
        )
        .await
        .unwrap();
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");
    assert_eq!(p2.get("name").unwrap(), "Project Charlie");
    assert_eq!(p2.get("priority").unwrap(), 1);

    let projects_name_asc = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            None,
            Some(&json!({"sort": [{"direction": "ascending", "orderBy": "name"}]})),
        )
        .await
        .unwrap();
    let projects_name_asc_a = projects_name_asc.as_array().unwrap();
    assert_eq!(projects_name_asc_a[0].get("name").unwrap(), "Project Alpha");
    assert_eq!(projects_name_asc_a[0].get("priority").unwrap(), 3);
    assert_eq!(projects_name_asc_a[1].get("name").unwrap(), "Project Bravo");
    assert_eq!(projects_name_asc_a[1].get("priority").unwrap(), 2);
    assert_eq!(
        projects_name_asc_a[2].get("name").unwrap(),
        "Project Charlie"
    );
    assert_eq!(projects_name_asc_a[2].get("priority").unwrap(), 1);

    let projects_name_desc = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            None,
            Some(&json!({"sort": [{"direction": "descending", "orderBy": "name"}]})),
        )
        .await
        .unwrap();
    let projects_name_desc_a = projects_name_desc.as_array().unwrap();
    assert_eq!(
        projects_name_desc_a[0].get("name").unwrap(),
        "Project Charlie"
    );
    assert_eq!(projects_name_desc_a[0].get("priority").unwrap(), 1);
    assert_eq!(
        projects_name_desc_a[1].get("name").unwrap(),
        "Project Bravo"
    );
    assert_eq!(projects_name_desc_a[1].get("priority").unwrap(), 2);
    assert_eq!(
        projects_name_desc_a[2].get("name").unwrap(),
        "Project Alpha"
    );
    assert_eq!(projects_name_desc_a[2].get("priority").unwrap(), 3);

    let projects_priority_asc = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            None,
            Some(&json!({"sort": [{"direction": "ascending", "orderBy": "priority"}]})),
        )
        .await
        .unwrap();
    let projects_priority_asc_a = projects_priority_asc.as_array().unwrap();
    assert_eq!(
        projects_priority_asc_a[0].get("name").unwrap(),
        "Project Charlie"
    );
    assert_eq!(projects_priority_asc_a[0].get("priority").unwrap(), 1);
    assert_eq!(
        projects_priority_asc_a[1].get("name").unwrap(),
        "Project Bravo"
    );
    assert_eq!(projects_priority_asc_a[1].get("priority").unwrap(), 2);
    assert_eq!(
        projects_priority_asc_a[2].get("name").unwrap(),
        "Project Alpha"
    );
    assert_eq!(projects_priority_asc_a[2].get("priority").unwrap(), 3);

    let projects_priority_desc = client
        .read_node(
            "Project",
            "__typename id name description status priority estimate active",
            None,
            Some(&json!({"sort": [{"direction": "descending", "orderBy": "priority"}]})),
        )
        .await
        .unwrap();
    let projects_priority_desc_a = projects_priority_desc.as_array().unwrap();
    assert_eq!(
        projects_priority_desc_a[0].get("name").unwrap(),
        "Project Alpha"
    );
    assert_eq!(projects_priority_desc_a[0].get("priority").unwrap(), 3);
    assert_eq!(
        projects_priority_desc_a[1].get("name").unwrap(),
        "Project Bravo"
    );
    assert_eq!(projects_priority_desc_a[1].get("priority").unwrap(), 2);
    assert_eq!(
        projects_priority_desc_a[2].get("name").unwrap(),
        "Project Charlie"
    );
    assert_eq!(projects_priority_desc_a[2].get("priority").unwrap(), 1);
}

/// Passes if resolvers can handle a shape that reads a property that is not
/// present on the model object.
#[wg_test]
#[allow(dead_code)]
async fn handle_missing_properties<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description",
            &json!({"name": "MJOLNIR"}),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert!(p0.get("description").unwrap().is_null());

    let projects = client
        .read_node("Project", "__typename id name description", None, None)
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

/// Passes if the update mutation succeeds with a target node selected by attribute
#[wg_test]
#[allow(dead_code)]
async fn update_mutation<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            &json!({"name": "Project1", "status": "PENDING"}),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            &json!({"status": "ACTIVE"}),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
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

/// Passes if the update mutation succeeds with a null match, meaning update all nodes
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mutation_null_query<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            &json!({"name": "Project1", "status": "PENDING"}),
            None,
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
            &json!({"name": "Project2", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", None, None)
        .await
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
            None,
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
        .read_node("Project", "__typename id name status", None, None)
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 2);
    assert_eq!(after_projects_a[0].get("status").unwrap(), "ACTIVE");
    assert_eq!(after_projects_a[1].get("status").unwrap(), "ACTIVE");
}

/// Passes if the delete mutation succeeds with a target node selected by attribute
#[wg_test]
#[allow(dead_code)]
async fn delete_mutation<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            &json!({"name": "Project1", "status": "PENDING"}),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
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
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(pd, 1);
    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
        )
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

/// Passes if the update mutation succeeds with a null match, meaning delete all nodes
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mutation_null_query<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name status",
            &json!({"name": "Project1", "status": "PENDING"}),
            None,
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
            &json!({"name": "Project2", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project2");
    assert_eq!(p1.get("status").unwrap(), "PENDING");

    let before_projects = client
        .read_node("Project", "__typename id name status", None, None)
        .await
        .unwrap();

    assert!(before_projects.is_array());
    let before_projects_a = before_projects.as_array().unwrap();
    assert_eq!(before_projects_a.len(), 2);

    let pd = client
        .delete_node("Project", None, None, None)
        .await
        .unwrap();

    assert_eq!(pd, 2);

    let after_projects = client
        .read_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "Project1"}})),
            None,
        )
        .await
        .unwrap();

    assert!(after_projects.is_array());
    let after_projects_a = after_projects.as_array().unwrap();
    assert_eq!(after_projects_a.len(), 0);
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn error_on_node_missing_id_cypher() {
    init();
    clear_db().await;

    let mut graph = bolt_transaction()
        .await
        .expect("Could not get database client.");
    graph
        .execute_query::<CypherRequestCtx>(
            "CREATE (n:Project { name: 'Project One' })".to_string(),
            HashMap::new(),
        )
        .await
        .expect("Expected successful query run.");

    let client = cypher_test_client("./tests/fixtures/minimal.yml").await;
    error_on_node_missing_id(client).await;
}

/// Passes if creating a node manually without an id throws an error upon access
/// to that node.  There is no GraphSON variant of this test, because GraphSON
/// data stores automatically assign a UUID id.
#[allow(dead_code)]
async fn error_on_node_missing_id<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let projects = client
        .read_node(
            "Project",
            "__typename id name",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_null());

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "Project One"}})),
            &json!({"status": "ACTIVE"}),
            None,
        )
        .await
        .unwrap();

    assert!(pu.is_null());

    // No error thrown for deletion of nodes, because the node ID isn't accessed during deletion.
    // Therefore, there is no delete_node test here.
}
