use actix::System;
use actix_http::error::Error;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpResponse, HttpServer};
use std::collections::HashMap;
use std::include_str;
use warpgrapher::{Engine, Config};
use warpgrapher::engine::neo4j::Neo4jEndpoint;
use warpgrapher::juniper::http::GraphQLRequest;
use warpgrapher::juniper::http::playground::playground_source;

#[derive(Clone)]
struct ActixServerAppData {
    engine: Engine<(), ()>,
}

async fn graphql(data: Data<ActixServerAppData>, req: Json<GraphQLRequest>) -> Result<HttpResponse, Error> {
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

#[allow(clippy::ptr_arg)]
pub fn run_actix_server(engine: Engine<(), ()>) {
    let sys = System::new("warpgrapher-quickstart");
    let app_data = ActixServerAppData { engine: engine };
    HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .route("/graphql", web::post().to(graphql))
            .route("/graphiql", web::get().to(|| {
                HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(playground_source(&"/graphql"))
            }))
    })
    .bind("127.0.0.1:5000")
    .expect("Failed to start server")
    .run();
    let _ = sys.run();
}

fn main() -> Result<(), Error> {

    // load warpgrapher config
    let config = Config::from_string(include_str!("./config.yml").to_string())
        .expect("Failed to load config file");

    // define database endpoint
    let db = Neo4jEndpoint::from_env("DB_URL").unwrap();

    // create warpgrapher engine
    let engine: Engine<(), ()> = Engine::new(config, db).build()
        .expect("Failed to build engine");

    // serve the warpgrapher engine on actix webserver
    println!("Warpgrapher quickstart app: http://localhost:5000/graphiql");
    run_actix_server(engine);

    Ok(())
}
