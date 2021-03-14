mod setup;

#[cfg(feature = "neo4j")]
use serde_json::json;
#[cfg(feature = "neo4j")]
use setup::{clear_db, init, neo4j_test_client_with_events};
#[cfg(feature = "neo4j")]
use warpgrapher::engine::events::{EventFacade, EventHandlerBag};
//#[cfg(feature = "neo4j")]
//use warpgrapher::engine::objects::{Node, Rel};
#[cfg(feature = "neo4j")]
use warpgrapher::engine::value::Value;
#[cfg(feature = "neo4j")]
use warpgrapher::juniper::BoxFuture;
#[cfg(feature = "neo4j")]
use warpgrapher::Error;
use warpgrapher::Client;
use std::collections::HashMap;
#[cfg(feature = "neo4j")]
type Rctx = setup::Neo4jRequestCtx;

// convenience function that will trigger event handler
async fn read_projects(client: &mut Client<Rctx>) -> Result<serde_json::Value, Error>{
    client
        .read_node(
            "Project",
            "id name description status",
            None,
            None,
        )
        .await
}

#[cfg(feature = "neo4j")]
fn mock_handler(r: Rctx, mut ef: EventFacade<Rctx>, _meta: HashMap<String, String>) -> BoxFuture<Result<Rctx, Error>> {
    Box::pin(async move {

        // create node
        let project = ef.create_node(
            "Project", 
            json!({"name": "Project00", "description": "lasers"}), 
            None
        ).await?;
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project00".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("lasers".to_string()));
        
        // create node
        let project = ef.create_node(
            "Project", 
            json!({"name": "Project01", "description": "shields"}), 
            None
        ).await?;
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project01".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("shields".to_string()));

        // update node
        let projects = ef.update_node(
            "Project", 
            json!({
                "MATCH": {"name": {"EQ": "Project00"}},
                "SET": {"description": "sharks"}
            }),
            None
        ).await?;
        let project = projects.first().unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project00".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("sharks".to_string()));

        // read nodes
        let projects = ef.read_nodes(
            "Project",
            json!({}),
            None,
        ).await?;
        assert_eq!(projects.len(), 2);
        let project = projects.iter().find(|n| n.fields().get("name").unwrap() == &Value::String("Project00".to_string())).unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project00".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("sharks".to_string()));
        let project = projects.iter().find(|n| n.fields().get("name").unwrap() == &Value::String("Project01".to_string())).unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project01".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("shields".to_string()));

        // delete node
        let dr = ef.delete_node(
            "Project",
            json!({
                "MATCH": {
                    "name": {
                        "EQ": "Project00"
                    }
                }
            }),
            None
        ).await?;
        assert_eq!(dr, 1);

        // read nodes
        let projects = ef.read_nodes(
            "Project",
            json!({}),
            None,
        ).await?;
        assert_eq!(projects.len(), 1);
        let project = projects.first().unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(project.fields().get("name").unwrap(), &Value::String("Project01".to_string()));
        assert_eq!(project.fields().get("description").unwrap(), &Value::String("shields".to_string()));
        
        Ok(r)
    })
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn test_event_facade_ops() {
    init();
    clear_db().await;
    let mut ehb = EventHandlerBag::new();
    ehb.register_before_request(mock_handler);
    let mut client = neo4j_test_client_with_events("./tests/fixtures/config.yml", ehb).await;
    let _result = read_projects(&mut client).await;
}
