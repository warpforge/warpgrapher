mod setup;

#[cfg(feature = "neo4j")]
use serde_json::json;
use setup::AppRequestCtx;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, neo4j_test_client_with_events};
use warpgrapher::engine::events::EventHandlerBag;
use warpgrapher::engine::objects::{Node, Rel};
use warpgrapher::engine::value::Value;
use warpgrapher::Error;

#[derive(Debug)]
struct TestError {}

impl std::error::Error for TestError {}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

fn bmef(_v: Value) -> Result<Value, Error> {
    Err(Error::UserDefinedError {
        source: Box::new(TestError {}),
    })
}

fn bqef(_v_opt: Option<Value>) -> Result<Option<Value>, Error> {
    Err(Error::UserDefinedError {
        source: Box::new(TestError {}),
    })
}

fn anef(_v: Vec<Node<AppRequestCtx>>) -> Result<Vec<Node<AppRequestCtx>>, Error> {
    Err(Error::UserDefinedError {
        source: Box::new(TestError {}),
    })
}

fn aref(_v: Vec<Rel<AppRequestCtx>>) -> Result<Vec<Rel<AppRequestCtx>>, Error> {
    Err(Error::UserDefinedError {
        source: Box::new(TestError {}),
    })
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_node_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_create("Project".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
        )
        .await
        .unwrap();

    assert!(p0.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_node_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_read("Project".to_string(), bqef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client
        .read_node("Project", "id status", Some("1234"), None)
        .await
        .unwrap();

    assert!(projects.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_node_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_update("Project".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
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
            Some("1234"),
            Some(&json!({"name": "MJOLNIR"})),
            &json!({"status": "ACTIVE"}),
        )
        .await
        .unwrap();

    assert!(pu.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_node_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_node_delete("Project".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
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
            Some("1234"),
            Some(&json!({"name": "MJOLNIR"})),
            None,
        )
        .await
        .unwrap();

    assert!(pd.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_node_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_create("Project".to_string(), anef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
        )
        .await
        .unwrap();

    assert!(p0.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_node_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_read("Project".to_string(), anef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
        )
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client
        .read_node("Project", "id status", Some("1234"), None)
        .await
        .unwrap();

    assert!(projects.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_node_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_update("Project".to_string(), anef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
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
            Some("1234"),
            Some(&json!({"name": "MJOLNIR"})),
            &json!({"status": "ACTIVE"}),
        )
        .await
        .unwrap();

    assert!(pu.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_node_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_node_delete("Project".to_string(), anef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            Some("1234"),
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
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
            Some("1234"),
            Some(&json!({"name": "MJOLNIR"})),
            None,
        )
        .await
        .unwrap();

    assert!(pd.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_rel_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_create("ProjectIssuesRel".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_rel_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_read("ProjectIssuesRel".to_string(), bqef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel(
            "Project",
            "issues",
            "id props { since }",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(rels.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_rel_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_update("ProjectIssuesRel".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id props { since }",
            Some("1234"),
            Some(&json!({"props": {"since": "2000"}})),
            &json!({"props": {"since": "2010"}}),
        )
        .await
        .unwrap();

    assert!(ru.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_before_rel_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_rel_delete("ProjectIssuesRel".to_string(), bmef);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some("1234"),
            Some(&json!({"props": {"since": "2010"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rd.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_rel_create_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_create("ProjectIssuesRel".to_string(), aref);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_rel_read_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_read("ProjectIssuesRel".to_string(), aref);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel(
            "Project",
            "issues",
            "id props { since }",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(rels.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_rel_update_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_update("ProjectIssuesRel".to_string(), aref);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id props { since }",
            Some("1234"),
            Some(&json!({"props": {"since": "2000"}})),
            &json!({"props": {"since": "2010"}}),
        )
        .await
        .unwrap();

    assert!(ru.is_null());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_after_rel_delete_handler() {
    init();
    clear_db().await;

    let mut ehb = EventHandlerBag::new();
    ehb.register_after_rel_delete("ProjectIssuesRel".to_string(), aref);

    let mut client = neo4j_test_client_with_events("./tests/fixtures/minimal.yml", ehb).await;

    client
        .create_node(
            "Project",
            "id name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", Some("1234"), &json!({"name": "Bug Zero"}))
        .await
        .unwrap();

    let results = client.create_rel("Project", "issues", "__typename id props { since } src { id name } dst { ...on Bug { id name } }", Some("1234"),
    &json!({"name": "Project Zero"}), &json!([{"props": {"since": "2000"}, "dst": {"Bug": {"$EXISTING": {"name": "Bug Zero"}}}}])).await.unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("props").unwrap().get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some("1234"),
            Some(&json!({"props": {"since": "2010"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rd.is_null());
}