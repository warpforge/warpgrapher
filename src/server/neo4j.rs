use crate::error::{Error, ErrorKind};
///! Temporary scaffolding for creating a flexible structure
///! for defining the neo4j database endpoint.
use std::env::var_os;

/// Represents the neo4j transport protocols
pub enum Neo4jTransportProtocol {
    HTTP,
    BOLT,
}

/// This struct will represents a configurable neo4j endpoint
/// and will contain several convenience functions to generate
/// or fetch the endpoint
pub struct Neo4jEndpoint {
    _host: String,
    _port: String,
    _path: String,
    _username: String,
    _password: String,
    _protocol: Neo4jTransportProtocol,
}

impl Neo4jEndpoint {
    // Attempts to pull database url endpoint from an environment variable.
    // If it fails, it returns a default value.
    pub fn from_env(var_name: &str) -> Result<String, Error> {
        match var_os(var_name) {
            None => Err(Error::new(
                ErrorKind::EnvironmentVariableNotFound(var_name.to_string()),
                None,
            )),
            Some(os) => match os.to_str() {
                None => Err(Error::new(
                    ErrorKind::EnvironmentVariableNotFound(var_name.to_string()),
                    None,
                )),
                Some(osstr) => Ok(osstr.to_owned()),
            },
        }
    }
}
