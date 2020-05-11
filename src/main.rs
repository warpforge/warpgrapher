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

use warpgrapher::engine::config::Config;
use warpgrapher::engine::neo4j::Neo4jEndpoint;
use warpgrapher::engine::Engine;

#[derive(Clone)]
struct AppData {
    engine: Engine,
}

impl AppData {
    fn new(engine: Engine) -> AppData {
        AppData { engine }
    }
}

async fn graphql(data: Data<AppData>, req: Json<GraphQLRequest>) -> Result<HttpResponse, Error> {
    let metadata: HashMap<String, String> = HashMap::new();

    let resp = &data.engine.execute(req.into_inner(), metadata);

    match resp {
        Ok(body) => Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body.to_string())),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(e.to_string())),
    }
}

async fn graphiql(_data: Data<AppData>) -> impl Responder {
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

    let cfn = matches.value_of("CONFIG").expect("Configuration required.");

    let config = Config::from_file(cfn.to_string()).expect("Could not load config file");

    let db = match Neo4jEndpoint::from_env("DB_URL") {
        Ok(db) => db,
        Err(_) => panic!("Unable to find Neo4jEndpoint"),
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

    let app_data = AppData::new(engine);

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
