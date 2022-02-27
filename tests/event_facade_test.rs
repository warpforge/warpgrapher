mod setup;

#[cfg(feature = "cypher")]
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::{clear_db, cypher_test_client_with_events, init};
#[cfg(feature = "cypher")]
use warpgrapher::engine::events::{EventFacade, EventHandlerBag};
//use warpgrapher::engine::objects::{Node, Rel};
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::QueryResult;
#[cfg(feature = "cypher")]
use std::collections::HashMap;
#[cfg(feature = "cypher")]
use warpgrapher::engine::value::Value;
#[cfg(feature = "cypher")]
use warpgrapher::juniper::BoxFuture;
#[cfg(feature = "cypher")]
use warpgrapher::Client;
#[cfg(feature = "cypher")]
use warpgrapher::Error;
#[cfg(feature = "cypher")]
type Rctx = setup::CypherRequestCtx;

// convenience function that will trigger event handler
#[cfg(feature = "cypher")]
async fn read_projects(client: &mut Client<Rctx>) -> Result<serde_json::Value, Error> {
    client
        .read_node("Project", "id name description status", None, None)
        .await
}

#[cfg(feature = "cypher")]
fn mock_handler(
    r: Rctx,
    mut ef: EventFacade<Rctx>,
    _meta: HashMap<String, String>,
) -> BoxFuture<Result<Rctx, Error>> {
    Box::pin(async move {
        // create node
        let project = ef
            .create_node(
                "Project",
                json!({"name": "Project00", "description": "lasers"}),
                None,
            )
            .await?;
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project00".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("lasers".to_string())
        );

        // create node
        let project = ef
            .create_node(
                "Project",
                json!({"name": "Project01", "description": "shields"}),
                None,
            )
            .await?;
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project01".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("shields".to_string())
        );

        // update node
        let projects = ef
            .update_node(
                "Project",
                json!({
                    "MATCH": {"name": {"EQ": "Project00"}},
                    "SET": {"description": "sharks"}
                }),
                None,
            )
            .await?;
        let project = projects.first().unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project00".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("sharks".to_string())
        );

        // read nodes
        let projects = ef.read_nodes("Project", json!({}), None).await?;
        assert_eq!(projects.len(), 2);
        let project = projects
            .iter()
            .find(|n| n.fields().get("name").unwrap() == &Value::String("Project00".to_string()))
            .unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project00".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("sharks".to_string())
        );
        let project = projects
            .iter()
            .find(|n| n.fields().get("name").unwrap() == &Value::String("Project01".to_string()))
            .unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project01".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("shields".to_string())
        );

        // delete node
        let dr = ef
            .delete_node(
                "Project",
                json!({
                    "MATCH": {
                        "name": {
                            "EQ": "Project00"
                        }
                    }
                }),
                None,
            )
            .await?;
        assert_eq!(dr, 1);

        // read nodes
        let projects = ef.read_nodes("Project", json!({}), None).await?;
        assert_eq!(projects.len(), 1);
        let project = projects.first().unwrap();
        assert_eq!(project.type_name(), "Project");
        assert_eq!(
            project.fields().get("name").unwrap(),
            &Value::String("Project01".to_string())
        );
        assert_eq!(
            project.fields().get("description").unwrap(),
            &Value::String("shields".to_string())
        );

        #[cfg(feature = "cypher")]
        let query =  "MATCH (p:Project) WHERE p.name = $project_name RETURN p".to_string();
        // #[cfg(feature = "gremlin")]
        // let query = "g.V().has('name', $project_name)".to_string();

        let mut params = HashMap::new();
        params.insert("project_name".to_string(), Value::String("Project01".to_string()));

        let result: QueryResult = ef.execute_query(
            query,
            params,
        ).await?;

        let projects = match result {
            QueryResult::Cypher(rs) => rs,
            _ => panic!("Expected Cypher result"),
        };

        let project = match projects.first().unwrap().fields().first().unwrap() {
            bolt_proto::Value::Node(n) => n,
            _ => panic!("Expected Node"),
        };

        println!("{:#?}", project);
        assert_eq!(
            project.properties().get("name").unwrap(),
            &bolt_proto::value::Value::String("Project01".to_string())
        );

        Ok(r)
    })
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn test_event_facade_ops() {
    init();
    clear_db().await;
    let mut ehb = EventHandlerBag::new();
    ehb.register_before_request(mock_handler);
    let mut client = cypher_test_client_with_events("./tests/fixtures/config.yml", ehb).await;
    let _result = read_projects(&mut client).await;
}
