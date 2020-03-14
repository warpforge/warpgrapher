mod setup;

use serde_json::json;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use serial_test::serial;
#[cfg(feature = "graphson2")]
use setup::server::test_server_graphson2;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
use setup::test_client;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use setup::{clear_db, init};

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn create_mnst_new_nodes_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_new_nodes();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn create_mnst_new_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_new_nodes();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
fn create_mnst_new_nodes() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", 
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "dst": { "Commit": { "NEW": { "hash": "11111" } } } } ] })
        )
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity0 = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity0.len(), 2);

    assert!(activity0
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity0
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));

    let p1 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project One", "activity": [ { "dst": { "Commit": { "NEW": { "hash": "22222" } } } }, { "dst": { "Commit": { "NEW": { "hash": "33333" } } } } ] })
        )
        .unwrap();

    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project One");

    assert!(p1.get("activity").unwrap().is_array());
    let activity1 = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity1.len(), 2);

    assert!(activity1
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity1
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity1
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
    assert!(activity1
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "33333"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 2);

    assert!(projects_a
        .iter()
        .any(|p| p.get("id").unwrap() == p0.get("id").unwrap()));
    assert!(projects_a
        .iter()
        .any(|p| p.get("name").unwrap() == "Project Zero"));
    assert!(projects_a
        .iter()
        .any(|p| p.get("id").unwrap() == p1.get("id").unwrap()));
    assert!(projects_a
        .iter()
        .any(|p| p.get("name").unwrap() == "Project One"));

    let p3 = &projects_a[0];
    assert!(p3.is_object());
    assert_eq!(p3.get("__typename").unwrap(), "Project");

    assert!(p3.get("activity").unwrap().is_array());
    let activity2 = p3.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity2.len(), 2);

    assert!(activity2
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity2
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));

    let p4 = &projects_a[1];
    assert!(p4.is_object());
    assert_eq!(p4.get("__typename").unwrap(), "Project");

    assert!(p4.get("activity").unwrap().is_array());
    let activity3 = p4.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity3.len(), 2);

    assert!(activity3
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity3
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn create_mnst_existing_nodes_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_existing_nodes();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn create_mnst_existing_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_existing_nodes();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[allow(clippy::cognitive_complexity, dead_code)]
fn create_mnst_existing_nodes() {
    let mut client = test_client();

    let c0 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "00000"}))
        .unwrap();
    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "11111"}))
        .unwrap();
    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "EXISTING": { "hash": "00000" } } } }, { "dst": { "Commit": {"EXISTING": { "hash": "11111" }}}} ] }))
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity0 = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity0.len(), 2);

    assert!(activity0
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity0
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());

    assert!(p1.get("activity").unwrap().is_array());
    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn read_mnst_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_by_rel_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn read_mnst_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_by_rel_props();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn read_mnst_by_rel_props() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"activity": {"props": {"repo": "Repo Zero"}}}))
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn read_mnst_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_by_dst_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn read_mnst_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_by_dst_props();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship dst object
#[allow(clippy::cognitive_complexity, dead_code)]
fn read_mnst_by_dst_props() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"activity": {"dst": {"Commit": {"hash": "11111"}}}}))
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnst_new_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_new_node();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn update_mnst_new_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_new_node();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnst_new_node() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"NEW": {"hash": "22222"}}}}}})
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 3);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnst_existing_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_existing_node();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn update_mnst_existing_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_existing_node();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnst_existing_node() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    let _c0 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "22222"}))
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"EXISTING": {"hash": "22222"}}}}}})
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 3);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnst_relationship_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_relationship();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn update_mnst_relationship_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_relationship();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnst_relationship() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"UPDATE": {"match": {"dst": {"Commit": {"hash": "00000"}}}, "update": {"props": {"repo": "Repo 0"}}}}})
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo 0"));
    assert!(activityu
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mnst_relationship_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_relationship_by_rel_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_mnst_relationship_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_relationship_by_rel_props();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by its properties
#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mnst_relationship_by_rel_props() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"DELETE": {"match": {"props": {"repo": "Repo Zero"}}}}})
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 1);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activityu
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mnst_relationship_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_relationship_by_dst_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_mnst_relationship_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_relationship_by_dst_props();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by the properties of the dst object
#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mnst_relationship_by_dst_props() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"DELETE": {"match": {"dst": {"Commit": {"hash": "00000"}}}}}})
        )
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 1);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activityu
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_node_by_mnst_rel_property_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_node_by_mnst_rel_property();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_node_by_mnst_rel_property_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_node_by_mnst_rel_property();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_node_by_mnst_rel_property() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activityu = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    client
        .delete_node(
            "Project",
            Some(&json!({"activity": {"dst": {"Commit": {"hash": "00000"}}}})),
            Some(&json!({"activity": [{"match": {}}]})),
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_node_by_mnst_dst_property_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_node_by_mnst_dst_property();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn delete_node_by_mnst_dst_property_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_node_by_mnst_dst_property();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of the dst object at a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_node_by_mnst_dst_property() {
    let mut client = test_client();

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
    .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activityu = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    client
        .delete_node(
            "Project",
            Some(&json!({"activity": {"dst": {"Commit": {"hash": "00000"}}}})),
            Some(&json!({"activity": [{"match": {}}]})),
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}
