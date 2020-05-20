mod setup;

use serde_json::json;
#[cfg(feature = "cosmos")]
use setup::cosmos_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(feature = "cosmos")]
use setup::server::test_server_cosmos;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use setup::{clear_db, init};
use warpgrapher::client::Client;

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_mnst_new_nodes_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    create_mnst_new_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnst_new_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnst_new_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_new_nodes(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_mnst_existing_nodes_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    create_mnst_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnst_existing_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnst_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_existing_nodes(mut client: Client) {
    let c0 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "00000"}),
        )
        .await
        .unwrap();
    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "11111"}),
        )
        .await
        .unwrap();
    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_mnst_by_rel_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    read_mnst_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnst_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnst_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_by_rel_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_mnst_by_dst_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    read_mnst_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnst_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnst_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship dst object
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_by_dst_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnst_new_node_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnst_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnst_new_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnst_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_new_node(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnst_existing_node_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnst_existing_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnst_existing_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnst_existing_node(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_existing_node(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let _c0 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "22222"}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnst_relationship_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnst_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnst_relationship_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnst_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_relationship(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "props": { "repo": "Repo Zero" }, "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "props": { "repo": "Repo One" },  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mnst_relationship_by_rel_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_mnst_relationship_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnst_relationship_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnst_relationship_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by its properties
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_relationship_by_rel_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mnst_relationship_by_dst_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_mnst_relationship_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnst_relationship_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnst_relationship_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a relationship by the properties of the dst object
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_relationship_by_dst_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_node_by_mnst_rel_property_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_node_by_mnst_rel_property(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_node_by_mnst_rel_property_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_node_by_mnst_rel_property(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnst_rel_property(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
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
            Some("1234"),
            Some(&json!({"activity": {"dst": {"Commit": {"hash": "00000"}}}})),
            Some(&json!({"activity": [{"match": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",  Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_node_by_mnst_dst_property_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_node_by_mnst_dst_property(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_node_by_mnst_dst_property_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_node_by_mnst_dst_property(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node by the properties of the dst object at a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnst_dst_property(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }",  Some("1234"),
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
            Some("1234"),
            Some(&json!({"activity": {"dst": {"Commit": {"hash": "00000"}}}})),
            Some(&json!({"activity": [{"match": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id props { repo } dst { ...on Commit { __typename id hash } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}
