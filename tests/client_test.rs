mod setup;

#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use serial_test::serial;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, test_client};

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn client_node_crud() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
        )
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client.read_node("Project", "id status", None).unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": "MJOLNIR"})),
            &json!({"status": "ACTIVE"}),
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "MJOLNIR");
    assert_eq!(pu_a[0].get("status").unwrap(), "ACTIVE");

    let u_projects = client.read_node("Project", "id status", None).unwrap();

    assert!(u_projects.is_array());
    let u_projects_a = u_projects.as_array().unwrap();
    assert_eq!(u_projects_a.len(), 1);
    assert_eq!(u_projects_a[0].get("status").unwrap(), "ACTIVE");

    let pd = client
        .delete_node("Project", Some(&json!({"name": "MJOLNIR"})), None)
        .unwrap();

    assert_eq!(pd, 1);

    let d_projects = client.read_node("Project", "id status", None).unwrap();

    assert!(d_projects.is_array());
    let d_projects_a = d_projects.as_array().unwrap();
    assert_eq!(d_projects_a.len(), 0);

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn client_rel_crud() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}))
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}))
        .unwrap();

    let results = client.create_rel("Project", "issues", "id props { since } src { id name } dst { ...on Bug { id name } }",
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"EXISTING": {"name": "Bug Zero"}}}}])).unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel("Project", "issues", "id props { since }", None)
        .unwrap();

    assert!(rels.is_array());
    let rels_a = rels.as_array().unwrap();
    assert_eq!(rels_a.len(), 1);
    assert_eq!(
        rels_a[0].get("props").unwrap().get("since").unwrap(),
        "2000"
    );

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id props { since }",
            Some(&json!({"props": {"since": "2000"}})),
            &json!({"props": {"since": "2010"}}),
        )
        .unwrap();

    assert!(ru.is_array());
    let ru_a = ru.as_array().unwrap();
    assert_eq!(ru_a.len(), 1);
    assert_eq!(ru_a[0].get("props").unwrap().get("since").unwrap(), "2010");

    let u_rels = client
        .read_rel("Project", "issues", "id props { since }", None)
        .unwrap();

    assert!(u_rels.is_array());
    let u_rels_a = u_rels.as_array().unwrap();
    assert_eq!(u_rels_a.len(), 1);
    assert_eq!(
        u_rels_a[0].get("props").unwrap().get("since").unwrap(),
        "2010"
    );

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some(&json!({"props": {"since": "2010"}})),
            None,
            None,
        )
        .unwrap();

    assert_eq!(rd, 1);

    let d_rels = client.read_rel("Project", "issues", "id", None).unwrap();

    assert!(d_rels.is_array());
    let d_rels_a = d_rels.as_array().unwrap();
    assert_eq!(d_rels_a.len(), 0);

    assert!(server.shutdown().is_ok());
}
