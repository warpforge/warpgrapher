mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_mnst_new_rel() {
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

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            &json!({"name": "Project Zero"}),
            &json!([{"props": {"repo": "Repo Zero"}, "dst": {"Commit": {"NEW": {"hash": "00000"}}}},
                    {"props": {"repo": "Repo One"}, "dst": {"Commit": {"NEW": {"hash": "11111"}}}}])
        )
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
            None,
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_mnst_rel_existing_node() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let _p0 = client
        .create_node("Project", "name", &json!({"name": "Project Zero"}))
        .unwrap();

    let c0 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "00000"}))
        .unwrap();

    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "11111"}))
        .unwrap();

    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            &json!({"name": "Project Zero"}),
            &json!([{"props": {"repo": "Repo Zero"}, "dst": {"Commit": {"EXISTING": {"hash": "00000"}}}},
                    {"props": {"repo": "Repo One"}, "dst": {"Commit": {"EXISTING": {"hash": "11111"}}}}])
        )
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
            None,
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnst_rel_by_rel_props() {
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
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some(&json!({"props": {"repo": "Repo Zero"}})),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnst_rel_by_src_props() {
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
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{ __typename hash}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_mnst_rel_by_dst_props() {
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
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some(&json!({"dst": {"Commit": {"hash": "00000"}}})),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnst_rel_by_rel_prop() {
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
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some(&json!({"props": {"repo": "Repo Zero"}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
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
            Some(&json!({"name": "Project Zero"})),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnst_rel_by_src_prop() {
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
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_mnst_rel_by_dst_prop() {
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
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename props{repo} dst{...on Commit{__typename hash}}",
            Some(&json!({"dst": {"Commit": {"hash": "00000"}}})),
            &json!({"props": {"repo": "Repo Two"}}),
        )
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
            Some(&json!({"name": "Project Zero"})),
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mnst_rel_by_rel_prop() {
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
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"props": {"repo": "Repo One"}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            None,
        )
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

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_mnst_rel_by_dst_prop() {
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
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"dst": {"Commit": {"hash": "11111"}}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            None,
        )
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
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
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
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename props{repo} dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": "Project One"})),
        )
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

    assert!(server.shutdown().is_ok());
}
