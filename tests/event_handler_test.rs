mod setup;

#[cfg(feature = "cypher")]
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::CypherRequestCtx;
#[cfg(feature = "cypher")]
use setup::{clear_db, cypher_test_client_with_events, init};
#[cfg(feature = "cypher")]
use std::collections::HashMap;
#[cfg(feature = "cypher")]
use warpgrapher::engine::events::{EventFacade, EventHandlerBag};
#[cfg(feature = "cypher")]
use warpgrapher::engine::objects::{Node, Rel};
#[cfg(feature = "cypher")]
use warpgrapher::engine::value::Value;
#[cfg(feature = "cypher")]
use warpgrapher::juniper::BoxFuture;
#[cfg(feature = "cypher")]
use warpgrapher::Error;
#[cfg(feature = "cypher")]
type Rctx = CypherRequestCtx;

#[derive(Debug)]
struct TestError {}

impl std::error::Error for TestError {}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

#[cfg(feature = "cypher")]
fn breqf(
    _r: Rctx,
    _ef: EventFacade<Rctx>,
    _meta: HashMap<String, String>,
) -> BoxFuture<Result<Rctx, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
fn areqf(
    _ef: EventFacade<Rctx>,
    _output: serde_json::Value,
) -> BoxFuture<Result<serde_json::Value, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
fn bmef(_v: Value, _ef: EventFacade<CypherRequestCtx>) -> BoxFuture<Result<Value, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
fn bqef(
    _v_opt: Option<Value>,
    _ef: EventFacade<CypherRequestCtx>,
) -> BoxFuture<Result<Option<Value>, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
fn anef(
    _v: Vec<Node<CypherRequestCtx>>,
    _ef: EventFacade<CypherRequestCtx>,
) -> BoxFuture<Result<Vec<Node<CypherRequestCtx>>, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
fn aref(
    _v: Vec<Rel<CypherRequestCtx>>,
    _ef: EventFacade<CypherRequestCtx>,
) -> BoxFuture<Result<Vec<Rel<CypherRequestCtx>>, Error>> {
    Box::pin(async move {
        Err(Error::UserDefinedError {
            source: Box::new(TestError {}),
        })
    })
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_request_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_request(breqf);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let result = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await;

    if let Error::UserDefinedError { source: _ } = result.err().unwrap() {
    } else {
        panic!("Unexpected error");
    }
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_request_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_request(areqf);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let result = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await;

    if let Error::UserDefinedError { source: _ } = result.err().unwrap() {
    } else {
        panic!("Unexpected error");
    }
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_node_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_create(vec!["Project".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_node_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_read(vec!["Project".to_string()], bqef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client
        .read_node("Project", "id status", None, None)
        .await
        .unwrap();

    assert!(projects.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_node_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_update(vec!["Project".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            &json!({"status": "ACTIVE"}),
            None,
        )
        .await
        .unwrap();

    assert!(pu.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_node_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_delete(vec!["Project".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");
    let pd = client
        .delete_node(
            "Project",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(pd.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_node_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_create(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_subgraph_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_subgraph_create(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_node_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_read(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client
        .read_node("Project", "id status", None, None)
        .await
        .unwrap();

    assert!(projects.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_node_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_update(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            &json!({"status": "ACTIVE"}),
            None,
        )
        .await
        .unwrap();

    assert!(pu.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_node_subgraph_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_subgraph_update(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            &json!({"status": "ACTIVE"}),
            None,
        )
        .await
        .unwrap();

    assert!(pu.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_node_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_delete(vec!["Project".to_string()], anef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");
    let pd = client
        .delete_node(
            "Project",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(pd.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_rel_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_create(vec!["ProjectIssuesRel".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_rel_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_read(vec!["ProjectIssuesRel".to_string()], bqef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel("Project", "issues", "id since", None, None)
        .await
        .unwrap();

    assert!(rels.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_rel_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_update(vec!["ProjectIssuesRel".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id since",
            Some(&json!({"since": {"EQ": "2000"}})),
            &json!({"since": "2010"}),
            None,
        )
        .await
        .unwrap();

    assert!(ru.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_before_rel_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_delete(vec!["ProjectIssuesRel".to_string()], bmef);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some(&json!({"since": {"EQ": "2010"}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rd.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_rel_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_create(vec!["ProjectIssuesRel".to_string()], aref);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_rel_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_read(vec!["ProjectIssuesRel".to_string()], aref);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}}]
            ),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel("Project", "issues", "id since", None, None)
        .await
        .unwrap();

    assert!(rels.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_rel_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_update(vec!["ProjectIssuesRel".to_string()], aref);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id since",
            Some(&json!({"since": {"EQ": "2000"}})),
            &json!({"since": "2010"}),
            None,
        )
        .await
        .unwrap();

    assert!(ru.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_rel_subgraph_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_subgraph_update(vec!["ProjectIssuesRel".to_string()], aref);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}
            }]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id since",
            Some(&json!({"since": {"EQ": "2000"}})),
            &json!({"since": "2010"}),
            None,
        )
        .await
        .unwrap();

    assert!(ru.is_null());
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_after_rel_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_delete(vec!["ProjectIssuesRel".to_string()], aref);

    let mut client = cypher_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "__typename id since src { id name } dst { ...on Bug { id name } }",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{
                "since": "2000",
                "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}}}}}]
            ),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some(&json!({"since": {"EQ": "2010"}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rd.is_null());
}
