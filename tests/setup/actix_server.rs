extern crate log;
extern crate serde_yaml;
extern crate warpgrapher;

use actix::System;
use actix_cors::Cors;
use actix_http::error::Error;
use actix_web::dev;
use actix_web::middleware::Logger;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpResponse, HttpServer};
use juniper::http::GraphQLRequest;
use std::collections::HashMap;
use std::sync::mpsc::Sender;

use super::server::{AppGlobalCtx, AppReqCtx};
use warpgrapher::engine::Engine;
use warpgrapher::engine::config::{WarpgrapherConfig, WarpgrapherValidators, WarpgrapherResolvers};
use warpgrapher::engine::extensions::WarpgrapherExtensions;

#[derive(Clone)]
struct AppData {
    engine: Engine<AppGlobalCtx, AppReqCtx>,
}

impl AppData {
    fn new(engine: Engine<AppGlobalCtx, AppReqCtx>) -> AppData {
        AppData { engine }
    }
}

async fn graphql(data: Data<AppData>, req: Json<GraphQLRequest>) -> Result<HttpResponse, Error> {
    let metadata: HashMap<String, String> = HashMap::new();

    let resp = &data.engine.execute(req, metadata);

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
pub fn start(
    config: &WarpgrapherConfig,
    db_url: &str,
    global_ctx: &AppGlobalCtx,
    resolvers: &WarpgrapherResolvers<AppGlobalCtx, AppReqCtx>,
    validators: &WarpgrapherValidators,
    extensions: &WarpgrapherExtensions<AppGlobalCtx, AppReqCtx>,
    tx: Sender<Result<dev::Server, warpgrapher::Error>>,
) {
    let engine = Engine::<AppGlobalCtx, AppReqCtx>::new(config.clone(), db_url.to_string())
        .with_version("1.0".to_string())
        .with_global_ctx(global_ctx.clone())
        .with_resolvers(resolvers.clone())
        .with_validators(validators.clone())
        .with_extensions(extensions.clone())
        .build()
        .expect("Could not create warpgrapher engine");

    let graphql_endpoint = "/graphql";
    let bind_addr = "127.0.0.1".to_string();
    let bind_port = "5000".to_string();
    let addr = format!("{}:{}", bind_addr, bind_port);

    let sys = System::new("warpgrapher-test-server");

    let app_data = AppData::new(engine);

    let srv = HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route(graphql_endpoint, web::post().to(graphql))
    })
    .bind(&addr)
    .map_err(|e| {
        let k = match e.kind() {
            std::io::ErrorKind::AddrInUse => warpgrapher::ErrorKind::AddrInUse(e),
            _ => warpgrapher::ErrorKind::AddrNotAvailable(e),
        };
        let _ = tx.send(Err(warpgrapher::Error::new(k, None)));
    })
    .unwrap();

    let server = srv.system_exit().run();
    let _ = tx.send(Ok(server));
    let _ = sys.run();
}
