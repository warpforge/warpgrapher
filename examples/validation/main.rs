use std::collections::HashMap;
use std::convert::TryFrom;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::engine::validators::Validators;
use warpgrapher::engine::value::Value;
use warpgrapher::{Engine, Error};

static CONFIG: &str = "
version: 1
model:
  # User
  - name: User
    props:
      - name: name
        type: String
        required: true
        validator: NameValidator
";

#[derive(Clone, Debug)]
struct AppRequestContext {}

impl RequestContext for AppRequestContext {
    type DBEndpointType = Neo4jEndpoint;
    fn new() -> AppRequestContext {
        AppRequestContext {}
    }
}

fn name_validator(value: &Value) -> Result<(), Error> {
    if let Value::Map(m) = value {
        if let Some(Value::String(name)) = m.get("name") {
            if name == "KENOBI" {
                Err(Error::ValidationFailed {
                    message: format!(
                        "Input validator for {field_name} failed. Cannot be named KENOBI",
                        field_name = "name"
                    ),
                })
            } else {
                Ok(())
            }
        } else {
            Err(Error::ValidationFailed {
                message: format!(
                    "Input validator for {field_name} failed.",
                    field_name = "name"
                ),
            })
        }
    } else {
        Err(Error::ValidationFailed {
            message: format!(
                "Input validator for {field_name} failed.",
                field_name = "name"
            ),
        })
    }
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

    // load validators
    let mut validators: Validators = Validators::new();
    validators.insert("NameValidator".to_string(), Box::new(name_validator));

    // create warpgrapher engine
    let engine: Engine<AppRequestContext> = Engine::new(config, db)
        .with_validators(validators.clone())
        .build()
        .expect("Failed to build engine");

    let query = "
        mutation {
            UserCreate(input: {
                name: \"KENOBI\"
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
