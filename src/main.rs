extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_yaml;
extern crate warpgrapher;

use clap::{App, Arg};
use warpgrapher::server::bind_port_from_env;
use warpgrapher::{Error, Neo4jEndpoint, Server};
use warpgrapher::config::{WarpgrapherConfig};

fn main() -> Result<(), Error> {
    env_logger::init();

    // define cli
    let matches = App::new("warpgrapher")
        .version("0.8")
        .about("WarpGrapher sample application.")
        .author("WarpGrapher")
        .arg(
            Arg::with_name("CONFIG")
                .help("Configuration file to use")
                .required(true),
        )
        .get_matches();

    let cfn = matches
        .value_of("CONFIG")
        .expect("Configuration filename required.");

    // load config
    let config = WarpgrapherConfig::from_file(cfn.to_string()).expect("Could not load config file");

    // define database endpoint
    let db = Neo4jEndpoint::from_env("DB_URL")?;

    // build server
    let mut server = Server::<(), ()>::new(config, db)
        .with_bind_port(bind_port_from_env("WG_SAMPLE_PORT"))
        .with_playground_endpoint("/graphiql".to_owned())
        .build()
        .expect("Error creating server");

    // run server
    println!(
        "Starting server on: http://{}:{}/graphiql",
        &server.bind_addr, &server.bind_port
    );
    match server.serve(true) {
        Ok(()) => {}
        Err(e) => error!("Server error: {}", e),
    };

    Ok(())
}
