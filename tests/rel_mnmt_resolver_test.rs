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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnmt_new_rel_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnmt_new_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_mnmt_new_rel_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    create_mnmt_new_rel(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_new_rel(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let i0 = client
        .create_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}", Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
            &json!([{"props": {"since": "today"}, "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}},
                    {"props": {"since": "yesterday"}, "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}}]),
        )
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));

    let projects = client
        .read_node(
            "Project",
            "issues {__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnmt_rel_existing_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnmt_rel_existing_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_mnmt_rel_existing_node_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    create_mnmt_rel_existing_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_rel_existing_node(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let b0 = client
        .create_node(
            "Bug",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Bug Zero"}),
        )
        .await
        .unwrap();

    assert!(b0.is_object());
    assert_eq!(b0.get("__typename").unwrap(), "Bug");
    assert_eq!(b0.get("name").unwrap(), "Bug Zero");

    let f0 = client
        .create_node(
            "Feature",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Feature Zero"}),
        )
        .await
        .unwrap();

    assert!(f0.is_object());
    assert_eq!(f0.get("__typename").unwrap(), "Feature");
    assert_eq!(f0.get("name").unwrap(), "Feature Zero");

    let i0 = client
        .create_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",  Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
            &json!([
                {"props": {"since": "today"}, "dst": {"Feature": {"EXISTING": {"name": "Feature Zero"}}}},
                {"props": {"since": "yesterday"}, "dst": {"Bug": {"EXISTING": {"name": "Bug Zero"}}}},
            ]))
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}}", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnmt_rel_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnmt_rel_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_mnmt_rel_by_rel_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    read_mnmt_rel_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_rel_by_rel_props(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                        "props": {"since": "yesterday"},
                        "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                        "props": {"since": "today"},
                        "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",Some("1234".to_string()),
            Some(&json!({"props": {"since": "yesterday"}})),
        )
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnmt_rel_by_src_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnmt_rel_by_src_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_mnmt_rel_by_src_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    read_mnmt_rel_by_src_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_rel_by_src_props(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                        "props": {"since": "yesterday"},
                        "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                        "props": {"since": "today"},
                        "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_mnmt_rel_by_dst_props_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    read_mnmt_rel_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_rel_by_dst_props(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                        "props": {"since": "yesterday"},
                        "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                        "props": {"since": "today"},
                        "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                        "props": {"since": "last week"},
                        "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    },
                    {
                        "props": {"since": "last month"},
                        "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}", Some("1234".to_string()),
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
        )
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));

    let i1 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}", Some("1234".to_string()),
            Some(&json!({"dst": {"Feature": {"name": "Feature Zero"}}})),
        )
        .await
        .unwrap();

    let issues = i1.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_rel_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnmt_rel_by_rel_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnmt_rel_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_rel_by_rel_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                      "props": {"since": "yesterday"},
                      "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                      "props": {"since": "today"},
                      "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                      "props": {"since": "last week"},
                      "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    },
                    {
                      "props": {"since": "last month"},
                      "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let i0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"props": {"since": "yesterday"}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
        .await
        .unwrap();

    let issues = i0.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "tomorrow"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}}", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 4);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "tomorrow"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last week"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last month"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_rel_by_src_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnmt_rel_by_src_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnmt_rel_by_src_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_rel_by_src_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                        "props": {"since": "yesterday"},
                        "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                        "props": {"since": "today"},
                        "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
        .await
        .unwrap();

    let issues = a0.as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "tomorrow"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "today"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_rel_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_mnmt_rel_by_dst_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    update_mnmt_rel_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_rel_by_dst_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                      "props": {"since": "yesterday"},
                      "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                      "props": {"since": "today"},
                      "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                      "props": {"since": "last week"},
                      "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    },
                    {
                      "props": {"since": "last month"},
                      "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
        .await
        .unwrap();

    let issues = a0.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "today"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() == "tomorrow"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 4);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "tomorrow"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last week"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last month"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnmt_rel_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mnmt_rel_by_rel_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_mnmt_rel_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnmt_rel_by_rel_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                      "props": {"since": "yesterday"},
                      "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                      "props": {"since": "today"},
                      "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                      "props": {"since": "last week"},
                      "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    },
                    {
                      "props": {"since": "last month"},
                      "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "issues",
            Some("1234".to_string()),
            Some(&json!({"props": {"since": "today"}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 3);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last week"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last month"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() != "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnmt_rel_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mnmt_rel_by_dst_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_mnmt_rel_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnmt_rel_by_dst_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                      "props": {"since": "yesterday"},
                      "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                      "props": {"since": "today"},
                      "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
                    },
                    {
                      "props": {"since": "last week"},
                      "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    },
                    {
                      "props": {"since": "last month"},
                      "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "issues",
            Some("1234".to_string()),
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 3);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .all(|i| i.get("props").unwrap().get("since").unwrap() != "today"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last week"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last month"));
    assert!(issues
        .iter()
        .all(|i| i.get("dst").unwrap().get("name").unwrap() != "Bug Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnmt_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnmt_rel_by_src_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_mnmt_rel_by_src_prop_cosmos() {
    init();
    clear_db();

    let mut server = test_server_cosmos("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = cosmos_test_client();
    delete_mnmt_rel_by_src_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnmt_rel_by_src_prop(mut client: Client) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({
                "name": "Project Zero",
                "issues": [
                    {
                        "props": {"since": "yesterday"},
                        "dst": {"Feature": {"NEW": {"name": "Feature Zero"}}}
                    },
                    {
                        "props": {"since": "today"},
                        "dst": {"Bug": {"NEW": {"name": "Bug Zero"}}}
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
                "issues": [
                    {
                        "props": {"since": "last week"},
                        "dst": {"Feature": {"NEW": {"name": "Feature One"}}}
                    },
                    {
                        "props": {"since": "last month"},
                        "dst": {"Bug": {"NEW": {"name": "Bug One"}}}
                    }
                ]
            }),
        )
        .await
        .unwrap();

    let _i0 = client
        .delete_rel(
            "Project",
            "issues",
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
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    let projects_a = projects0.as_array().unwrap();
    let project = &projects_a[0];
    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 0);

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let issues = project.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(issues.iter().all(|i| i.is_object()));
    assert!(issues
        .iter()
        .all(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last week"));
    assert!(issues
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "last month"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));
}
