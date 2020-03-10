extern crate log;
extern crate serde_yaml;
extern crate warpgrapher;

use actix::System;
use actix_cors::Cors;
use actix_http::error::Error;
use actix_web::middleware::Logger;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpResponse, HttpServer};
use juniper::http::GraphQLRequest;
use std::collections::HashMap;

use super::{GlobalContext, ReqContext};
use warpgrapher::Engine;

#[derive(Clone)]
struct AppData {
    engine: Engine<GlobalContext, ReqContext>
}

impl AppData {
    fn new(
        engine: Engine<GlobalContext, ReqContext>
    ) -> AppData {
        AppData {
            engine
        }
    }
}

/*
struct Headers {
    data: HashMap<String, String>
}

impl Headers {
    fn new() -> Headers {
        Headers {
            data: HashMap::new(),
        }
    }
}

impl FromRequest for Headers {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = HeadersConfig;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let mut h = Headers::new();
        h.data = HashMap::new();
  
        for (k, v) in req.headers().iter() {
            if let Ok(s) = v.to_str() {
                h.data.insert(k.as_str().to_string(), s.to_string());
            }
        }

        ok(h)
    }
}

struct HeadersConfig {
    _ehandler: Option<Arc<dyn Fn(QueryPayloadError, &HttpRequest) -> Error + Send + Sync>>,
}

impl Default for HeadersConfig {
    fn default() -> Self {
        HeadersConfig { _ehandler: None }
    }
}
*/

async fn graphql(
      data: Data<AppData>,
      req: Json<GraphQLRequest>,
      //_headers: Headers,
    ) -> Result<HttpResponse, Error> {
 
    //TODO Convert actix Json to serde_json

    //TODO Convert headers to metadata Hashmap
    let metadata: HashMap<String, String> = HashMap::new();

    let resp = &data.engine.execute(req, metadata);

    match resp {
          Ok(body) => {
              Ok(HttpResponse::Ok()
                  .content_type("application/json")
                  .body(body.to_string()))
          },
          Err(e) => {
              Ok(HttpResponse::InternalServerError()
                  .content_type("application/json")
                  .body(e.to_string()))
          }
    }
    
}

#[allow(clippy::ptr_arg)]
pub fn start(
    engine: Engine<GlobalContext, ReqContext>,
) {
    let graphql_endpoint = "/graphql";
    let bind_addr = "127.0.0.1".to_string();
    let bind_port = "5000".to_string();
    let addr = format!("{}:{}", bind_addr, bind_port);

    let sys = System::new("warpgrapher-example-server");

    let app_data = AppData::new(
        engine
    );

    HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route(graphql_endpoint, web::post().to(graphql))
    })
    .bind(&addr)
    .expect("Failed to start server")
    .run();

    println!("Server available on: {:#?}", &addr);
    let _ = sys.run();
}
