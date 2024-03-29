mod setup;

use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snmt_new_rel<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project One"}),
            None,
        )
        .await
        .unwrap();

    let b0a = client
        .create_rel(
            "Project",
            "board",
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"publicized": true, "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}}),
            None
        )
        .await
        .unwrap();

    assert!(b0a.is_array());
    assert_eq!(b0a.as_array().unwrap().len(), 1);
    let b0 = b0a.as_array().unwrap().iter().next().unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("publicized").unwrap() == true);

    let b1a = client
        .create_rel(
            "Project",
            "board",
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": {"EQ": "Project One"}}),
            &json!({"publicized": false, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
            None
        )
        .await
        .unwrap();

    assert!(b1a.is_array());
    assert_eq!(b1a.as_array().unwrap().len(), 1);
    let b1 = b1a.as_array().unwrap().iter().next().unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("publicized").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None
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
    assert!(board.get("publicized").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None
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
    assert!(board.get("publicized").unwrap() == false);
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snmt_rel_existing_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project One"}),
            None,
        )
        .await
        .unwrap();

    let _s0 = client
        .create_node(
            "ScrumBoard",
            "__typename name",
            &json!({"name": "ScrumBoard Zero"}),
            None,
        )
        .await
        .unwrap();

    let _k0 = client
        .create_node(
            "KanbanBoard",
            "__typename name",
            &json!({"name": "KanbanBoard Zero"}),
            None,
        )
        .await
        .unwrap();

    let b0a = client
        .create_rel(
            "Project",
            "board",
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({
                "publicized": true, 
                "dst": {"KanbanBoard": {"EXISTING": {"name": {"EQ": "KanbanBoard Zero"}}}}
            }),
            None
        )
        .await
        .unwrap();

    assert!(b0a.is_array());
    assert_eq!(b0a.as_array().unwrap().len(), 1);
    let b0 = b0a.as_array().unwrap().iter().next().unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("publicized").unwrap() == true);

    let b1a = client
        .create_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            &json!({"name": {"EQ": "Project One"}}),
            &json!({
                "publicized": false, 
                "dst": {"ScrumBoard": {"EXISTING": {"name": {"EQ": "ScrumBoard Zero"}}}}
            }),
            None
        )
        .await
        .unwrap();

    assert!(b1a.is_array());
    assert_eq!(b1a.as_array().unwrap().len(), 1);
    let b1 = b1a.as_array().unwrap().iter().next().unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("publicized").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(board.get("publicized").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let board = project.get("board").unwrap();

    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(board.get("publicized").unwrap() == false);
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
            None,
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
                    "publicized": false,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            Some(&json!({"publicized": true})),
            None
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == true));
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
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{ __typename name}}",
            Some(&json!({"publicized": false})),
            None
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_src_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
            None,
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
                    "publicized": false,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().any(|b| b.get("publicized").unwrap() == true));
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
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", 
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project One"}}}})),
            None
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
            None,
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
                    "publicized": false,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .read_rel(
            "Project",
            "board",
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            Some(&json!({"dst": {"ScrumBoard": {"name": {"EQ": "ScrumBoard Zero"}}}})),
            None
        )
        .await
        .unwrap();

    let board = b0.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == true));
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
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", 
            Some(&json!({"dst": {"KanbanBoard": {"name": {"EQ": "KanbanBoard Zero"}}}})),
            None
        )
        .await
        .unwrap();

    let board = b1.as_array().unwrap();
    assert_eq!(board.len(), 1);

    assert!(board.iter().all(|b| b.is_object()));
    assert!(board
        .iter()
        .all(|b| b.get("__typename").unwrap() == "ProjectBoardRel"));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                  "publicized": true,
                  "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
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
                  "publicized": false,
                  "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"publicized": true})),
            &json!({"publicized": false}),
            None
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
    assert!(board.iter().any(|b| b.get("publicized").unwrap() == false));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != true));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", 
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let p1 = &projects_a[0];
    let board = p1.get("board").unwrap();

    assert!(board.is_object());
    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(board.get("publicized").unwrap() != true);
    assert!(board.get("publicized").unwrap() == false);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"publicized": false})),
            &json!({"publicized": true}),
            None
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
    assert!(board.iter().any(|b| b.get("publicized").unwrap() == true));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != false));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", 
            Some(&json!({"name": {"EQ": "Project One"}})),
            None
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let p1 = &projects_a[0];
    let board = p1.get("board").unwrap();

    assert!(board.is_object());
    assert!(board.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(board.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(board.get("publicized").unwrap() != false);
    assert!(board.get("publicized").unwrap() == true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                }
            }),
            None,
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
                    "publicized": false,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            &json!({"publicized": false}),
            None
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
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == false));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project One"}}}})),
            &json!({"publicized": true}),
            None
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
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == true));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": false,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                  }
            }),
            None,
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
                    "publicized": true,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}
                  }
            }),
            None,
        )
        .await
        .unwrap();

    let b0 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", 
            Some(&json!({"dst": {"KanbanBoard": {"name": {"EQ": "KanbanBoard Zero"}}}})),
            &json!({"publicized": true}),
            None
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
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != false));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename publicized dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}",
            Some(&json!({"dst": {"ScrumBoard": {"name": {"EQ": "ScrumBoard Zero"}}}})),
            &json!({"publicized": false}),
            None
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
    assert!(board.iter().all(|b| b.get("publicized").unwrap() != true));
    assert!(board.iter().all(|b| b.get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                      "publicized": true,
                      "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                    }
            }),
            None,
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"publicized": true})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
            None
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
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"publicized": false, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
            None
        )
        .await
        .unwrap();

    let _b2 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"publicized": false})),
            None,
            None,
            None,
        )
        .await;

    let projects = client
        .read_node(
            "Project",
            "board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
            None
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
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"dst": {"KanbanBoard": {"name": {"EQ": "KanbanBoard Zero"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
            None
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
            "__typename publicized dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"publicized": false, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
            None
        )
        .await
        .unwrap();

    let _b2 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"dst": {"ScrumBoard": {"name": {"EQ": "ScrumBoard Zero"}}}})),
            None,
            None,
            None,
        )
        .await;

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            None,
            None
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
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "board": {
                    "publicized": true,
                    "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}
                }
            }),
            None,
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
                    "publicized": false,
                    "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard One"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _b0 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "__typename name board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None
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
    assert!(board.get("publicized").unwrap() == false);
    assert!(board.get("publicized").unwrap() != true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard One");
    assert!(board.get("dst").unwrap().get("name").unwrap() != "KanbanBoard One");

    let _b1 = client
        .delete_rel(
            "Project",
            "board",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project One"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects2 = client
        .read_node(
            "Project",
            "__typename name board{__typename publicized dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None
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
}
