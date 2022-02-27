# AWS Lambda

To integrate Warpgrapher with AWS lambda, begin by including the following dependencies in `Cargo.toml`. 

`Cargo.toml`

```toml
[dependencies]
lambda_runtime = "0.3.0"
serde = "1.0.57"
serde_json = "1.0.57"
serde_derive = "1.0.57"
tokio = { version="1.4.0", features=["rt-multi-thread", "macros"] }
warpgrapher = { version="0.10.2", features = ["gremlin"] }
```

In the `main.rs` source file, include the following code to include structs and functions that are needed from dependencies.

```
use api_service::{create_app_engine, Error};
use lambda_runtime::handler_fn;
use serde_derive::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env::var_os;
use std::sync::Arc;
use warpgrapher::engine::database::gremlin::GremlinEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::juniper::BoxFuture;
```

Next the lambda integration defines a `GraphqlRequest` struct that is used to deserialize query strings and request variables from the lambda interface for passing to the Warpgrapher engine.

```
#[derive(Clone, Debug, Deserialize)]
pub struct GraphqlRequest {
    pub query: String,
    pub variables: Option<Value>,
}
```

The `AwsLambdaProxyRequest` struct is used to deserialize requests incoming from AWS lambda. Within the body of the request is the content that will be deserialized into the `GraphqlRequest` struct described above.

```
#[derive(Clone, Debug, Deserialize)]
pub struct AwsLambdaProxyRequest {
    pub body: String,
    #[serde(rename = "requestContext")]
    pub request_context: AwsLambdaProxyRequestContext,
}
```

The `aws_proxy_response` function below packages a result returned by a Warpgrapher engine's `execute` function into a format that can be returned to the AWS lambda framework.

```
pub fn aws_proxy_response(body: serde_json::Value) -> Result<JSON, Error> {
    Ok(json!({
      "body": serde_json::to_string(&body)
        .map_err(|e| Error::JsonSerializationError { source: e})?,
      "headers": json!({}),
      "isBase64Encoded": false,
      "statusCode": 200
    }))
}
```

The `create_app_engine` function takes a database pool, Gremlin in this example, and returns a Warpgrapher `Engine` that can be used to handle GraphQL queries.

```
    static CONFIG: &str = " version: 1
      model:
        - name: User
          props:
            - name: email
              type: String
    ";

    // create config
    let config = Configuration::try_from(CONFIG.to_string()).expect("Failed to parse CONFIG");

    // create warpgrapher engine
    let engine: Engine<Rctx> = Engine::<Rctx>::new(config, db).build()?;

    Ok(engine)
}
```

The `main` function ties the above elements together to process a GraphQL query when the lambda function is invoked. The function creates a database pool from environment variables, as described in the [Databases](./configuration/databases.html) section of the book. The `main` function then uses the `create_app_engine` function to create a Warpgrapher `Engine`. A closure is defined that deserializes the request from the AWS lambda function and passes it to the Warpgrapher engine for execution using the `execute` method.  The results are packaged up for response using the `aws_proxy_response` method.  That handler closure is passed to the lambda runtime for invocation when requests need to be processed.

```
#[tokio::main]
async fn main() -> Result<(), Error> {
    // define database endpoint
    let endpoint = GremlinEndpoint::from_env()?;
    let db = endpoint.pool().await?;

    // create warpgrapher engine
    let engine = Arc::new(create_app_engine(db).await?);

    let func = handler_fn(
        move |event: Value, _: lambda_runtime::Context| -> BoxFuture<Result<Value, Error>> {
            let eng = engine.clone();

            Box::pin(async move {
                let engine = eng.clone();

                // parse handler event as aws proxy request and extract graphql request
                let proxy_request = serde_json::from_value(event).map_err(|e| 
                    Error::JsonDeserializationError {
                        desc: "Failed to deserialize aws proxy request".to_string(),
                        source: e,
                    })?;
                let gql_request = serde_json::from_str(&proxy_request.body).map_err(|e| 
                    Error::JsonDeserializationError {
                        desc: "Failed to deserialize graphql request in body".to_string(),
                        source: e,
                    })?;

                // execute request
                let result = engine
                    .execute(
                        gql_request.query.to_string(),
                        gql_request.variables,
                        HashMap::new(),
                    )
                    .await?;

                // format response for api-gateway proxy
                aws_proxy_response(result)
                    .or_else(|e| aws_proxy_response(json!({ "errors": [format!("{}", e)] })))
            })
        },
    );

    lambda_runtime::run(func)
        .await
        .map_err(|_| Error::LambdaError {})?;
    Ok(())
}
```
