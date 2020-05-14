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

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn create_mnst_new_rel_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_new_rel().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn create_mnst_new_rel_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_new_rel().await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_new_rel() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}", Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
            &json!([{"props": {"repo": "Repo Zero"}, "dst": {"Commit": {"NEW": {"hash": "00000"}}}},
                    {"props": {"repo": "Repo One"}, "dst": {"Commit": {"NEW": {"hash": "11111"}}}}])
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn create_mnst_rel_existing_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_rel_existing_node().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn create_mnst_rel_existing_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnst_rel_existing_node().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_rel_existing_node() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        ).await
        .unwrap();

    let c0 = client
        .create_node(
            "Commit",
            "__typename hash",
            Some("1234".to_string()),
            &json!({"hash": "00000"}),
        ).await
        .unwrap();

    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node(
            "Commit",
            "__typename hash",
            Some("1234".to_string()),
            &json!({"hash": "11111"}),
        ).await
        .unwrap();

    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
            &json!([{"props": {"repo": "Repo Zero"}, "dst": {"Commit": {"EXISTING": {"hash": "00000"}}}},
                    {"props": {"repo": "Repo One"}, "dst": {"Commit": {"EXISTING": {"hash": "11111"}}}}])
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn read_mnst_rel_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_rel_props().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn read_mnst_rel_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_rel_props().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_rel_props() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"props": {"repo": "Repo Zero"}})),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn read_mnst_rel_by_src_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_src_props().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn read_mnst_rel_by_src_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_src_props().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_src_props() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{ __typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
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

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn read_mnst_rel_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_dst_props().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn read_mnst_rel_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnst_rel_by_dst_props().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_dst_props() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"dst": {"Commit": {"hash": "00000"}}})),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn update_mnst_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_rel_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn update_mnst_rel_by_rel_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_rel_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_rel_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"props": {"repo": "Repo Zero"}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn update_mnst_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_src_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn update_mnst_rel_by_src_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_src_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_src_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn update_mnst_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_dst_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn update_mnst_rel_by_dst_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnst_rel_by_dst_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_dst_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some("1234".to_string()),
            Some(&json!({"dst": {"Commit": {"hash": "00000"}}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn delete_mnst_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_rel_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn delete_mnst_rel_by_rel_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_rel_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_rel_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                      "props": {"repo": "Repo Zero"},
                      "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                      "props": {"repo": "Repo One"},
                      "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some("1234".to_string()),
            Some(&json!({"props": {"repo": "Repo One"}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "11111"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn delete_mnst_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_dst_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn delete_mnst_rel_by_dst_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_dst_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_dst_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                      "props": {"repo": "Repo Zero"},
                      "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                      "props": {"repo": "Repo One"},
                      "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some("1234".to_string()),
            Some(&json!({"dst": {"Commit": {"hash": "11111"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("props").unwrap().get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "11111"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
#[serial(neo4j)]
async fn delete_mnst_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_src_prop().await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
#[serial(graphson2)]
async fn delete_mnst_rel_by_src_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnst_rel_by_src_prop().await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_src_prop() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "props": {"repo": "Repo Zero"},
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "props": {"repo": "Repo One"},
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project One",
                "activity": [
                    {
                        "props": {"repo": "Repo Two"},
                        "dst": {"Commit": {"NEW": {"hash": "22222"}}}
                    },
                    {
                        "props": {"repo": "Repo Three"},
                        "dst": {"Commit": {"NEW": {"hash": "33333"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    let projects_a = projects0.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 0);

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
    assert!(activity
        .iter()
        .any(|a| a.get("props").unwrap().get("repo").unwrap() == "Repo Three"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "33333"));
}
