mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn create_mnst_new_nodes() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", 
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "dst": { "Commit": { "NEW": { "hash": "11111" } } } } ] })
        )
        .await
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
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn create_mnst_existing_nodes() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let c0 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "00000"}))
        .await
        .unwrap();
    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "11111"}))
        .await
        .unwrap();
    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "EXISTING": { "hash": "00000" } } } }, { "dst": { "Commit": {"EXISTING": { "hash": "11111" }}}} ] }))
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn read_mnst_by_rel_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship dst object
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn read_mnst_by_dst_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_mnst_new_node() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"NEW": {"hash": "22222"}}}}}})
        )
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_mnst_existing_node() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let _c0 = client
        .create_node("Commit", "__typename id hash", &json!({"hash": "22222"}))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"EXISTING": {"hash": "22222"}}}}}})
        )
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_mnst_relationship() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            Some(&json!({"name": "Project Zero"})),
            &json!({"activity": {"UPDATE": {"match": {"dst": {"Commit": {"hash": "00000"}}}, "update": {"props": {"repo": "Repo 0"}}}}})
        )
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by its properties
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_mnst_relationship_by_rel_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by the properties of the dst object
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_mnst_relationship_by_dst_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
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

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_node_by_mnst_rel_property() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of the dst object at a relationship
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_node_by_mnst_dst_property() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");

    assert!(server.serve(false).is_ok());

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
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
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);

    assert!(server.shutdown().is_ok());
}
