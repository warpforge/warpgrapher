mod setup;

use serde_json::json;
#[cfg(feature = "cosmos")]
use setup::cosmos_test_client;
#[cfg(feature = "gremlin")]
use setup::gremlin_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use setup::{clear_db, init};
use setup::{AppGlobalCtx, AppRequestCtx};
use warpgrapher::client::Client;

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snmt_new_rel_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_new_rel(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snmt_new_rel_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_new_rel(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snmt_new_rel_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_new_rel(client).await;
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snmt_new_rel(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project One"}),
        )
        .await
        .unwrap();

    let b0a = client
        .create_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project Zero"}),
            &json!({"props": {"publicized": true}, "dst": {"KanbanBoard": {"NEW": {"name": "KanbanBoard Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(b0a.is_array());
    assert_eq!(b0a.as_array().unwrap().len(), 1);
    let b0 = b0a.as_array().unwrap().iter().next().unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("props").unwrap().get("publicized").unwrap() == true);

    let b1a = client
        .create_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project One"}),
            &json!({"props": {"publicized": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(b1a.is_array());
    assert_eq!(b1a.as_array().unwrap().len(), 1);
    let b1 = b1a.as_array().unwrap().iter().next().unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("props").unwrap().get("publicized").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() == false);
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snmt_new_existing_node_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_rel_existing_node(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snmt_rel_existing_node_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_rel_existing_node(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snmt_rel_existing_node_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    create_snmt_rel_existing_node(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snmt_rel_existing_node(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project One"}),
        )
        .await
        .unwrap();

    let _s0 = client
        .create_node(
            "ScrumBoard",
            "__typename name",
            Some("1234"),
            &json!({"name": "ScrumBoard Zero"}),
        )
        .await
        .unwrap();

    let _k0 = client
        .create_node(
            "KanbanBoard",
            "__typename name",
            Some("1234"),
            &json!({"name": "KanbanBoard Zero"}),
        )
        .await
        .unwrap();

    let b0a = client
        .create_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project Zero"}),
            &json!({
                "props": {"publicized": true}, 
                "dst": {"KanbanBoard": {"EXISTING": {"name": "KanbanBoard Zero"}}}
            }))
        .await
        .unwrap();

    assert!(b0a.is_array());
    assert_eq!(b0a.as_array().unwrap().len(), 1);
    let b0 = b0a.as_array().unwrap().iter().next().unwrap();

    assert!(b0.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b0.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard");
    assert!(b0.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");
    assert!(b0.get("props").unwrap().get("publicized").unwrap() == true);

    let b1a = client
        .create_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project One"}),
            &json!({
                "props": {"publicized": false}, 
                "dst": {"ScrumBoard": {"EXISTING": {"name": "ScrumBoard Zero"}}}
            }))
        .await
        .unwrap();

    assert!(b1a.is_array());
    assert_eq!(b1a.as_array().unwrap().len(), 1);
    let b1 = b1a.as_array().unwrap().iter().next().unwrap();

    assert!(b1.get("__typename").unwrap() == "ProjectBoardRel");
    assert!(b1.get("dst").unwrap().get("__typename").unwrap() == "ScrumBoard");
    assert!(b1.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
    assert!(b1.get("props").unwrap().get("publicized").unwrap() == false);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() == true);

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() == false);
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snmt_rel_by_rel_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_rel_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snmt_rel_by_rel_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_rel_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_snmt_rel_by_rel_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_rel_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_rel_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": false},
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            Some(&json!({"props": {"publicized": true}})),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{ __typename name}}", Some("1234"),
            Some(&json!({"props": {"publicized": false}})),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snmt_rel_by_src_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_src_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snmt_rel_by_src_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_src_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_snmt_rel_by_src_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_src_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_src_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": false},
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
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
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
        .any(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
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
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snmt_rel_by_dst_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_dst_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snmt_rel_by_dst_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_dst_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_snmt_rel_by_dst_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snmt_rel_by_dst_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snmt_rel_by_dst_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": false},
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("__typename").unwrap() == "KanbanBoard"));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_rel_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snmt_rel_by_rel_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_rel_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_snmt_rel_by_rel_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_rel_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_rel_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                  "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                  "props": {"publicized": false},
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
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"props": {"publicized": true}})),
            &json!({"props": {"publicized": false}}),
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
        .any(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != true));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() != true);
    assert!(board.get("props").unwrap().get("publicized").unwrap() == false);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero");

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"props": {"publicized": false}})),
            &json!({"props": {"publicized": true}}),
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
        .any(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != false));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
    assert!(board
        .iter()
        .any(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() != false);
    assert!(board.get("props").unwrap().get("publicized").unwrap() == true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snmt_rel_by_src_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_src_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snmt_rel_by_src_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_src_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_snmt_rel_by_src_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_src_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_src_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": false},
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
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            &json!({"props": {"publicized": false}}),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"src": {"Project": {"name": "Project One"}}})),
            &json!({"props": {"publicized": true}}),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_dst_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snmt_rel_by_dst_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_dst_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_snmt_rel_by_dst_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snmt_rel_by_dst_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snmt_rel_by_dst_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": false},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": true},
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
            "__typename props {publicized} dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"dst": {"KanbanBoard": {"name": "KanbanBoard Zero"}}})),
            &json!({"props": {"publicized": true}}),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != false));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == true));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "KanbanBoard Zero"));

    let b1 = client
        .update_rel(
            "Project",
            "board",
            "__typename props {publicized} dst {...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}", Some("1234"),
            Some(&json!({"dst": {"ScrumBoard": {"name": "ScrumBoard Zero"}}})),
            &json!({"props": {"publicized": false}}),
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
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() != true));
    assert!(board
        .iter()
        .all(|b| b.get("props").unwrap().get("publicized").unwrap() == false));
    assert!(board
        .iter()
        .all(|b| b.get("dst").unwrap().get("name").unwrap() == "ScrumBoard Zero"));
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snmt_rel_by_rel_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_rel_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snmt_rel_by_rel_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_rel_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snmt_rel_by_rel_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_rel_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_rel_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                      "props": {"publicized": true},
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
            Some("1234"),
            Some(&json!({"props": {"publicized": true}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project Zero"}),
            &json!({"props": {"publicized": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    let _b2 = client
        .delete_rel(
            "Project",
            "board",
            Some("1234"),
            Some(&json!({"props": {"publicized": false}})),
            None,
            None,
        )
        .await;

    let projects = client
        .read_node(
            "Project",
            "board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snmt_rel_by_dst_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_dst_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snmt_rel_by_dst_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_dst_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snmt_rel_by_dst_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_dst_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_dst_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            Some(&json!({"dst": {"KanbanBoard": {"name": "KanbanBoard Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
            "__typename props{publicized} dst{...on KanbanBoard{__typename name} ...on ScrumBoard{__typename name}}", Some("1234"),
            &json!({"name": "Project Zero"}),
            &json!({"props": {"publicized": false}, "dst": {"ScrumBoard": {"NEW": {"name": "ScrumBoard Zero"}}}}),
        )
        .await
        .unwrap();

    let _b2 = client
        .delete_rel(
            "Project",
            "board",
            Some("1234"),
            Some(&json!({"dst": {"ScrumBoard": {"name": "ScrumBoard Zero"}}})),
            None,
            None,
        )
        .await;

    let projects = client
        .read_node(
            "Project",
            "__typename name board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snmt_rel_by_src_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_src_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snmt_rel_by_src_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_src_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snmt_rel_by_src_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snmt_rel_by_src_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snmt_rel_by_src_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "board": {
                    "props": {"publicized": true},
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "board": {
                    "props": {"publicized": false},
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
            Some("1234"),
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
    assert!(board.get("props").unwrap().get("publicized").unwrap() == false);
    assert!(board.get("props").unwrap().get("publicized").unwrap() != true);
    assert!(board.get("dst").unwrap().get("name").unwrap() == "ScrumBoard One");
    assert!(board.get("dst").unwrap().get("name").unwrap() != "KanbanBoard One");

    let _b1 = client
        .delete_rel(
            "Project",
            "board",
            Some("1234"),
            Some(&json!({"src": {"Project": {"name": "Project One"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects2 = client
        .read_node(
            "Project",
            "__typename name board{__typename props{publicized} dst{...on ScrumBoard{__typename name} ...on KanbanBoard{__typename name}}}", Some("1234"),
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
}
