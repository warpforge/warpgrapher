use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use warpgrapher::engine::config::{Configuration, Property, UsesFilter};
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::CrudOperation;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::engine::events::{EventFacade, EventHandlerBag};
use warpgrapher::engine::objects::Node;
use warpgrapher::engine::value::Value;
use warpgrapher::juniper::BoxFuture;
use warpgrapher::{Engine, Error};

static CONFIG: &str = "
version: 1
model:
  - name: Record
    props:
      - name: content
        type: String
";

#[derive(Clone, Debug)]
pub struct Rctx {
    pub username: String,
}

impl Rctx {}

impl RequestContext for Rctx {
    type DBEndpointType = Neo4jEndpoint;

    fn new() -> Self {
        Rctx {
            username: String::new(),
        }
    }
}

/// This event handler executes at the beginning of every request and attempts to insert the
/// current user's profile into the request context.
fn insert_user_profile(
    mut rctx: Rctx,
    mut _ef: EventFacade<Rctx>,
    _metadata: HashMap<String, String>,
) -> BoxFuture<Result<Rctx, Error>> {
    Box::pin(async move {
        // A real implementation would likely pull a user identity from an authentication token in
        // metadata, or use that token to look up a full user profile in a database. In this
        // example, the identify is hard-coded.
        rctx.username = "user-from-JWT".to_string();
        Ok(rctx)
    })
}

/// before_build_engine event hook
/// Adds owner meta fields to all types in the model (though in this example, there's only one,
/// the record type)
fn add_owner_field(config: &mut Configuration) -> Result<(), Error> {
    for t in config.model.iter_mut() {
        let mut_props: &mut Vec<Property> = t.mut_props();
        mut_props.push(Property::new(
            "owner".to_string(),
            UsesFilter::none(),
            "String".to_string(),
            false,
            false,
            None,
            None,
        ));
    }
    Ok(())
}

/// before_create event hook
/// Inserts an owner meta property into every new node containing the id of the creator
fn insert_owner(mut v: Value, ef: EventFacade<'_, Rctx>) -> BoxFuture<Result<Value, Error>> {
    Box::pin(async move {
        if let CrudOperation::CreateNode(_) = ef.op() {
            if let Value::Map(ref mut input) = v {
                let user_name = ef
                    .context()
                    .request_context()
                    .expect("Expect context")
                    .username
                    .to_string();
                input.insert("owner".to_string(), Value::String(user_name));
            }
        }
        Ok(v)
    })
}

/// after_read event hook
/// Filters the read nodes to those that are authorized to be read
fn enforce_read_access(
    mut nodes: Vec<Node<Rctx>>,
    ef: EventFacade<'_, Rctx>,
) -> BoxFuture<Result<Vec<Node<Rctx>>, Error>> {
    Box::pin(async move {
        nodes.retain(|node| {
            let node_owner: String = node
                .fields()
                .get("owner")
                .unwrap()
                .clone()
                .try_into()
                .expect("Expect to find owner field.");

            node_owner
                == ef
                    .context()
                    .request_context()
                    .expect("Context expected")
                    .username
        });
        Ok(nodes)
    })
}

/// before_update event hook
/// Filters out nodes that the user is not authorized to modify
fn enforce_write_access(
    v: Value,
    mut ef: EventFacade<'_, Rctx>,
) -> BoxFuture<Result<Value, Error>> {
    Box::pin(async move {
        if let Value::Map(mut m) = v.clone() {
            if let Some(input_match) = m.remove("MATCH") {
                let nodes = &ef.read_nodes("Record", input_match, None).await?;

                // filter nodes that are authorized
                let filtered_node_ids: Vec<Value> = nodes
                    .iter()
                    .filter(|n| {
                        let node_owner: String =
                            n.fields().get("owner").unwrap().clone().try_into().unwrap();

                        node_owner
                            == ef
                                .context()
                                .request_context()
                                .expect("Expect context.")
                                .username
                    })
                    .map(|n| Ok(n.id()?.clone()))
                    .collect::<Result<Vec<Value>, Error>>()?;

                // replace MATCH input with filtered nodes
                m.insert(
                    "MATCH".to_string(),
                    Value::Map(hashmap! {
                        "id".to_string() => Value::Map(hashmap! {
                            "IN".to_string() => Value::Array(filtered_node_ids)
                        })
                    }),
                );

                // return modified input
                Ok(Value::Map(m))
            } else {
                // Return original input unmodified
                Ok(v)
            }
        } else {
            // Return original input unmodified
            Ok(v)
        }
    })
}

#[tokio::main]
async fn main() {
    // parse warpgrapher config
    let config = Configuration::try_from(CONFIG.to_string()).expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Neo4jEndpoint::from_env()
        .expect("Failed to parse neo4j endpoint from environment")
        .pool()
        .await
        .expect("Failed to create neo4j database pool");

    let mut ehb = EventHandlerBag::new();
    ehb.register_before_request(insert_user_profile);
    ehb.register_before_engine_build(add_owner_field);
    ehb.register_before_node_create(vec!["Record".to_string()], insert_owner);
    ehb.register_after_node_read(vec!["Record".to_string()], enforce_read_access);
    ehb.register_before_node_update(vec!["Record".to_string()], enforce_write_access);
    ehb.register_before_node_delete(vec!["Record".to_string()], enforce_write_access);

    // create warpgrapher engine
    let engine: Engine<Rctx> = Engine::new(config, db)
        .with_event_handlers(ehb)
        .build()
        .expect("Failed to build engine");

    let query = "
        mutation {
            RecordCreate(input: {
                content: \"Test Content\"
            }) {
                id
                name
            }
        }
    "
    .to_string();
    let metadata = HashMap::new();
    let result = engine.execute(query, None, metadata).await.unwrap();

    println!("result: {:#?}", result);
}
