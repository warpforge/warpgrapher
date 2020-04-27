mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn create_snmt_new_rel() {
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
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project One"}),
        )
        .await
        .unwrap();

    let b0 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({"props": {"public": true}, "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("props").unwrap().get("public").unwrap() == true);

    let b1 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": "Project One"}),
            &json!({"props": {"public": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("props").unwrap().get("public").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("board").unwrap().is_object());
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(board.get("props").unwrap().get("public").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("board").unwrap().is_object());
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(board.get("props").unwrap().get("public").unwrap() == false);

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn create_snmt_rel_existing_node() {
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
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project One"}),
        )
        .await
        .unwrap();

    let _s0 = client
        .create_node(
            "ScrumBoard",
            "__typename name",
            &json!({"name": "ScrumBoard Zero"}),
        )
        .await
        .unwrap();

    let _k0 = client
        .create_node(
            "KanbanBoard",
            "__typename name",
            &json!({"name": "KanbanBoard Zero"}),
        )
        .await
        .unwrap();

    let b0 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({
                "props": {"public": true}, 
                "dst": {"KanbanBoard": {"EXISTING": {"name": "KanbanBoard Zero"}}}
            }))
        .await
        .unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("props").unwrap().get("public").unwrap() == true);

    let b1 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            &json!({"name": "Project One"}),
            &json!({
                "props": {"public": false}, 
                "dst": {"ScrumBoard": {"EXISTING": {"name": "ScrumBoard Zero"}}}
            }))
        .await
        .unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("props").unwrap().get("public").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(board.get("props").unwrap().get("public").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(board.get("props").unwrap().get("public").unwrap() == false);

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn read_snmt_rel_by_rel_props() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": false},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            Some(&json!({"props": {"public": true}})),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{ __typename name}}",
            Some(&json!({"props": {"public": false}})),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn read_snmt_rel_by_src_props() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": false},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .any(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project One"}}})),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn read_snmt_rel_by_dst_props() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": false},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            Some(&json!({"dst": {"ScrumBoard": {"name": "ScrumBoard Zero"}}})),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .read_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            Some(&json!({"dst": {"KanbanBoard": {"name": "KanbanBoard Zero"}}})),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_snmt_rel_by_rel_prop() {
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
                "board": {
                  "props": {"public": true},
                  "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                  "props": {"public": false},
                  "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"props": {"public": true}})),
            &json!({"props": {"public": false}}),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .any(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != true));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let p1 = &projects_a[0];
    let board = p1.get("board").unwrap();

    assert!(board.is_object());
    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(board.get("props").unwrap().get("public").unwrap() != true);
    assert!(board.get("props").unwrap().get("public").unwrap() == false);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"props": {"public": false}})),
            &json!({"props": {"public": true}}),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 2);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .any(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != false));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let p1 = &projects_a[0];
    let board = p1.get("board").unwrap();

    assert!(board.is_object());
    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("props").unwrap().get("public").unwrap() != false);
    assert!(board.get("props").unwrap().get("public").unwrap() == true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_snmt_rel_by_src_prop() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": false},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"public": false}}),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": "Project One"}}})),
            &json!({"props": {"public": true}}),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn update_snmt_rel_by_dst_prop() {
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
                "board": {
                    "props": {"public": false},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                  }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": true},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                  }
            }),
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename props {public} dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"dst": {"KanbanBoard": {"name": "KanbanBoard Zero"}}})),
            &json!({"props": {"public": true}}),
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props {public} dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"dst": {"ScrumBoard": {"name": "ScrumBoard Zero"}}})),
            &json!({"props": {"public": false}}),
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() != true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("public").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_snmt_rel_by_rel_prop() {
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
                "board": {
                      "props": {"public": true},
                      "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                    }
            }),
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"props": {"public": true}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    let _b1 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({"props": {"public": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    let _b2 = client.delete_rel(
        "Project",
        "board",
        Some(&json!({"props": {"public": false}})),
        None,
        None,
    ).await;

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_snmt_rel_by_dst_prop() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"dst": {"KanbanBoard": {"name": "KanbanBoard Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    let _b1 = client
        .create_rel(
            "Project",
            "board",
            "__typename props{public} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": "Project Zero"}),
            &json!({"props": {"public": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    let _b2 = client.delete_rel(
        "Project",
        "board",
        Some(&json!({"dst": {"ScrumBoard": {"name": "ScrumBoard Zero"}}})),
        None,
        None,
    ).await;

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity)]
#[tokio::test]
#[serial]
async fn delete_mnst_rel_by_src_prop() {
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
                "board": {
                    "props": {"public": true},
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"public": false},
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard One"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    assert!(projects0.is_array());
    let projects_a = projects0.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(projects1.is_array());
    let projects_a = projects1.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    let board = project.get("board").unwrap();

    assert!(board.is_object());
    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() != "KanbanBoard");
    assert!(board.get("props").unwrap().get("public").unwrap() == false);
    assert!(board.get("props").unwrap().get("public").unwrap() != true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard One");
    assert!(board.get("dst").unwrap().get("name").unwrap() != "KanbanBoard One");

    let _b1 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"src": {"Project": {"name": "Project One"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects2 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{public} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    assert!(projects2.is_array());
    let projects_a = projects2.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("board").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(server.shutdown().is_ok());
}
