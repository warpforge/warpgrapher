extern crate log;
extern crate serde_yaml;
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

use super::{AppGlobalContext, AppRequestContext};
use warpgrapher::engine::Engine;

#[derive(Clone)]
struct AppData {
    engine: Engine<AppGlobalContext, AppRequestContext>,
}

impl AppData {
    fn new(engine: Engine<AppGlobalContext, AppRequestContext>) -> AppData {
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

#[allow(clippy::ptr_arg)]
pub fn start(engine: Engine<AppGlobalContext, AppRequestContext>) {
    let graphql_endpoint = "/graphql";
    let bind_addr = "127.0.0.1".to_string();
    let bind_port = "5000".to_string();
    let addr = format!("{}:{}", bind_addr, bind_port);

    let sys = System::new("warpgrapher-example-server");

    let app_data = AppData::new(engine);

    HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route(graphql_endpoint, web::post().to(graphql))
            .route("/graphiql", web::get().to(graphiql))
    })
    .bind(&addr)
    .expect("Failed to start server")
    .run();

    println!("Server available on: {:#?}", &addr);
    let _ = sys.run();
}
