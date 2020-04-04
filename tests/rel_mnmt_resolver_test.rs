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
#[serial(neo4j)]
#[test]
fn create_mnmt_new_rel_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnmt_new_rel();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn create_mnmt_new_rel_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnmt_new_rel();

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
fn create_mnmt_new_rel() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        )
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
#[serial(neo4j)]
#[test]
fn create_mnmt_rel_existing_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnmt_rel_existing_node();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn create_mnmt_rel_existing_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    create_mnmt_rel_existing_node();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn create_mnmt_rel_existing_node() {
    let mut client = test_client();

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Project Zero"}),
        )
        .unwrap();

    let b0 = client
        .create_node(
            "Bug",
            "__typename name",
            Some("1234".to_string()),
            &json!({"name": "Bug Zero"}),
        )
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
#[serial(neo4j)]
#[test]
fn read_mnmt_rel_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_rel_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn read_mnmt_rel_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_rel_props();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn read_mnmt_rel_by_rel_props() {
    let mut client = test_client();

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
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",Some("1234".to_string()),
            Some(json!({"props": {"since": "yesterday"}})),
        )
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
#[serial(neo4j)]
#[test]
fn read_mnmt_rel_by_src_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_src_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn read_mnmt_rel_by_src_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_src_props();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn read_mnmt_rel_by_src_props() {
    let mut client = test_client();

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
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
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

#[cfg(feature = "neo4j")]
#[serial(neo4j)]
#[test]
fn read_mnmt_rel_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_dst_props();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn read_mnmt_rel_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    read_mnmt_rel_by_dst_props();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn read_mnmt_rel_by_dst_props() {
    let mut client = test_client();

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
        .unwrap();

    let i0 = client
        .read_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}", Some("1234".to_string()),
            Some(json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
        )
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
            Some(json!({"dst": {"Feature": {"name": "Feature Zero"}}})),
        )
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
#[serial(neo4j)]
#[test]
fn update_mnmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_rel_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnmt_rel_by_rel_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_rel_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnmt_rel_by_rel_prop() {
    let mut client = test_client();

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
        .unwrap();

    let i0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"props": {"since": "yesterday"}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
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
            Some(json!({"name": "Project Zero"})),
        )
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
#[serial(neo4j)]
#[test]
fn update_mnmt_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_src_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnmt_rel_by_src_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_src_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnmt_rel_by_src_prop() {
    let mut client = test_client();

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
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
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
#[serial(neo4j)]
#[test]
fn update_mnmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_dst_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn update_mnmt_rel_by_dst_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    update_mnmt_rel_by_dst_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn update_mnmt_rel_by_dst_prop() {
    let mut client = test_client();

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
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}", Some("1234".to_string()),
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
            &json!({"props": {"since": "tomorrow"}}),
        )
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
            Some(json!({"name": "Project Zero"})),
        )
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
#[serial(neo4j)]
#[test]
fn delete_mnmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_rel_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mnmt_rel_by_rel_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_rel_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mnmt_rel_by_rel_prop() {
    let mut client = test_client();

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
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            None,
        )
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
#[serial(neo4j)]
#[test]
fn delete_mnmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_dst_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mnmt_rel_by_dst_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_dst_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mnmt_rel_by_dst_prop() {
    let mut client = test_client();

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
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            None,
        )
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
#[serial(neo4j)]
#[test]
fn delete_mnmt_rel_by_src_prop_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_src_prop();

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[serial(graphson2)]
#[test]
fn delete_mnmt_rel_by_src_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    delete_mnmt_rel_by_src_prop();

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
fn delete_mnmt_rel_by_src_prop() {
    let mut client = test_client();

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
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            Some(json!({"name": "Project Zero"})),
        )
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}", Some("1234".to_string()),
            Some(json!({"name": "Project One"})),
        )
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
