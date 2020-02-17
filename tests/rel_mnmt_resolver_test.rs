mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_mnmt_new_rel() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
        )
        .unwrap();

    let i0 = client
        .create_rel(
            "Project",
            "issues",
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",
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
            "issues {__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_mnmt_rel_existing_node() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
        )
        .unwrap();

    let b0 = client
        .create_node("Bug", "__typename name", &json!({"name": "Bug Zero"}))
        .unwrap();

    assert!(b0.is_object());
    assert_eq!(b0.get("__typename").unwrap(), "Bug");
    assert_eq!(b0.get("name").unwrap(), "Bug Zero");

    let f0 = client
        .create_node(
            "Feature",
            "__typename name",
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
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",
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
            "__typename name issues{__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}}",
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnmt_rel_by_rel_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",
            Some(&json!({"props": {"since": "yesterday"}})),
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnmt_rel_by_src_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnmt_rel_by_dst_props() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
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
            "__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}",
            Some(&json!({"dst": {"Feature": {"name": "Feature Zero"}}})),
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnmt_rel_by_rel_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}",
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
            "__typename name issues{__typename props{since} dst{...on Feature{__typename name} ...on Bug{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnmt_rel_by_src_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}",
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnmt_rel_by_dst_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
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
            "__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}",
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
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mnmt_rel_by_rel_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            Some(&json!({"props": {"since": "today"}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mnmt_rel_by_dst_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            Some(&json!({"dst": {"Bug": {"name": "Bug Zero"}}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mnst_rel_by_src_prop() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
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
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name issues{__typename props{since} dst{...on Bug{__typename name} ...on Feature{__typename name}}}",
            Some(&json!({"name": "Project One"})),
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

    assert!(server.shutdown().is_ok());
}
