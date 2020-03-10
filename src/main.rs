extern crate env_logger;
extern crate warpgrapher;

use actix::System;
use actix_cors::Cors;
use actix_http::error::Error;
use actix_web::middleware::Logger;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use juniper::http::playground::playground_source;
use juniper::http::GraphQLRequest;
use std::collections::HashMap;

use warpgrapher::{Neo4jEndpoint, Engine, WarpgrapherConfig};

#[derive(Clone)]
struct AppData {
    engine: Engine
}

impl AppData {
    fn new(
        engine: Engine 
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

async fn graphiql(
        _data: Data<AppData>, 
    ) -> impl Responder {

    let html = playground_source(&"/graphql");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[allow(clippy::match_wild_err_arm)]
fn main() {
    env_logger::init();
    
    let matches = clap::App::new("warpgrapher-actixweb")
        .version("0.1")
        .about("Warpgrapher sample application using actix-web server")
        .author("Warpgrapher")
        .arg(
            clap::Arg::with_name("CONFIG")
                .help("Path to configuration file to use")
                .required(true),
        )
        .get_matches();

    let cfn = matches
        .value_of("CONFIG")
        .expect("Configuration required.");

    let config = WarpgrapherConfig::from_file(cfn.to_string())
        .expect("Could not load config file");

    let db = match Neo4jEndpoint::from_env("DB_URL") {
        Ok(db) => db,
        Err(_) => panic!("Unable to find Neo4jEndpoint")
    }; 
    
    let engine = Engine::<(), ()>::new(config, db)
        .with_version("1.0".to_string())
        .build()
        .expect("Could not create warpgrapher engine");

    let graphql_endpoint = "/graphql";
    let playground_endpoint = "/graphiql";
    let bind_addr = "127.0.0.1".to_string();
    let bind_port = "5000".to_string();
    let addr = format!("{}:{}", bind_addr, bind_port);

    let sys = System::new("warpgrapher-actixweb");

    let app_data = AppData::new(
        engine
    );

    HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route(graphql_endpoint, web::post().to(graphql))
            .route(playground_endpoint, web::get().to(graphiql))
    })
    .bind(&addr)
    .expect("Failed to start server")
    .run();

    println!("Server available on: {:#?}", &addr);
    let _ = sys.run();
}
