use resolvers::{project_count, project_points};
use validators::name_validator;
use warpgrapher::Error;
use warpgrapher::engine::Engine;
use warpgrapher::engine::neo4j::Neo4jEndpoint;
use warpgrapher::engine::config::{WarpgrapherConfig, WarpgrapherResolvers, WarpgrapherValidators};
use warpgrapher::engine::context::WarpgrapherRequestContext;

extern crate env_logger;
extern crate frank_jwt;
extern crate log;
extern crate warpgrapher;

mod actix_server;
mod resolvers;
mod validators;

#[derive(Clone, Debug)]
pub struct GlobalContext {
    s3_client: String,
}

#[derive(Clone, Debug, Default)]
pub struct ReqContext {
    //pub user: Option<UserProfile>,
}

impl ReqContext {
    pub fn new() -> ReqContext {
        ReqContext { /*user: None*/ }
    }
}

/*
impl JwtAuthReqContext for ReqContext {
    fn set_user(&mut self, user: UserProfile) {
        self.user = Some(user)
    }
}
*/

impl WarpgrapherRequestContext for ReqContext {
    fn new() -> ReqContext {
        ReqContext::new()
    }
}

fn main() -> Result<(), Error> {
    // initialize logging
    env_logger::init();

    // context
    let global_ctx = GlobalContext {
        s3_client: "https://s3.aws.com".to_string(),
    };

    // resolvers
    let mut resolvers = WarpgrapherResolvers::<GlobalContext, ReqContext>::new();
    resolvers.insert(
        "ProjectCount".to_string(),
        Box::new(project_count::resolver),
    );
    resolvers.insert(
        "ProjectPoints".to_string(),
        Box::new(project_points::resolver),
    );

    let mut validators = WarpgrapherValidators::new();

    validators.insert(
        "NameValidator".to_string(),
        Box::new(name_validator::validator),
    );

    /*   TEMPORARY REMOVAL UNTIL CRATE IS PUBLISHED
    // extensions
    let jwtauth: JwtAuthExtension<GlobalContext, ReqContext> =
        JwtAuthExtension::new(vec![Arc::new(BasicAuthProvider::new(
            "test.com".to_string(),
            "secret".to_string(),
        ))]);
    */

    // config
    let config = WarpgrapherConfig::from_file("./examples/project-tracker/config.yml".to_string())
        .expect("Failed to load config file");

    // define database endpoint
    let db = Neo4jEndpoint::from_env("DB_URL")?;

    // engine
    let engine: Engine<GlobalContext, ReqContext> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .with_validators(validators)
        .with_global_ctx(global_ctx)
        .build()
        .expect("Failed to build engine");

    actix_server::start(engine);

    Ok(())
}
