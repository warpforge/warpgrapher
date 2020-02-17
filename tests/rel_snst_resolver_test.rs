mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_snst_new_rel() {
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

    let o0 = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({"props": {"since": "yesterday"}, "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
        )
        .unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("props").unwrap().get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("owner").unwrap().is_object());
    let owner = project.get("owner").unwrap();

    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_snst_rel_existing_node() {
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

    let _u0 = client
        .create_node("User", "__typename name", &json!({"name": "User Zero"}))
        .unwrap();

    let o0 = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({
                "props": {"since": "yesterday"},
                "dst": {"User": {"EXISTING": {"name": "User Zero"}}}
            }),
        )
        .unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("props").unwrap().get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_snst_rel_by_rel_props() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some(&json!({"props": {"since": "yesterday"}})),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_snst_rel_by_src_props() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .any(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_snst_rel_by_dst_props() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_snst_rel_by_rel_prop() {
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
                "owner": {
                  "props": {"since": "yesterday"},
                  "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some(&json!({"props": {"since": "yesterday"}})),
            &json!({"props": {"since": "today"}}),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .any(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .any(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_snst_rel_by_src_prop() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"since": "today"}}),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    let projects_a = projects.as_array().unwrap();

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_snst_rel_by_dst_prop() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                  }
            }),
        )
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props {since} dst {...on User{__typename name}}",
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
            &json!({"props": {"since": "today"}}),
        )
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    let projects_a = projects.as_array().unwrap();

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_snst_rel_by_rel_prop() {
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
                "owner": {
                      "props": {"since": "yesterday"},
                      "dst": {"User": {"NEW": {"name": "User Zero"}}}
                    }
            }),
        )
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some(&json!({"props": {"since": "yesterday"}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_snst_rel_by_dst_prop() {
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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
            None,
            None,
        )
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            None,
        )
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

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
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "owner": {
                    "props": {"since": "today"},
                    "dst": {"User": {"NEW": {"name": "User One"}}}
                }
            }),
        )
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .unwrap();

    assert!(projects0.is_array());
    let projects_a = projects0.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(projects1.is_array());
    let projects_a = projects1.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User One");

    assert!(server.shutdown().is_ok());
}
