//! Warpgrapher makes it painless to create web services with graph-based data
//! models. Describe the data model for which you want to run a web service.
//! Wargrapher automatically generates a GraphQL schema from the data model, as
//! well as a set of resolvers for basic create, read, update, and delete (CRUD)
//! operations on that data. If you need more more sophisticated endpoints, you
//! can supply your own custom resolvers. Warpgrapher will automatically
//! generate the GraphQL configuration and invoke your custom resolvers when
//! appropriate.

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
#[macro_use]
extern crate juniper;

pub use juniper::{Arguments, ExecutionResult, Executor, Object, Value};

pub use client::WarpgrapherClient;
pub use error::{Error, ErrorKind};

pub use server::config::{
    WarpgrapherConfig, WarpgrapherResolverFunc, WarpgrapherResolvers, WarpgrapherValidatorFunc,
    WarpgrapherValidators,
};
pub use server::context::{GraphQLContext, WarpgrapherRequestContext};
#[cfg(feature = "graphson2")]
pub use server::database::graphson2::Graphson2Endpoint;
#[cfg(feature = "neo4j")]
pub use server::database::neo4j::Neo4jEndpoint;
pub use server::extensions::{Extension, WarpgrapherExtensions};
pub use server::schema::{Info, Property};
pub use server::{bind_addr_from_env, bind_port_from_env, Server};

pub mod client;
pub mod error;
pub mod server;
