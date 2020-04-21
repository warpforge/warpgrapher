//! Warpgrapher makes it painless to create web services with graph-based data
//! models. Describe the data model for which you want to run a web service.
//! Wargrapher automatically generates a GraphQL schema from the data model, as
//! well as a set of resolvers for basic create, read, update, and delete (CRUD)
//! operations on that data. If you need more more sophisticated endpoints, you
//! can supply your own custom resolvers. Warpgrapher will automatically
//! generate the GraphQL configuration and invoke your custom resolvers when
//! appropriate.

#[macro_use]
pub extern crate juniper;

pub use juniper::{Arguments, ExecutionResult, Executor, Object, Value};

pub use client::WarpgrapherClient;
pub use error::Error;
pub use error::ErrorKind;

pub use server::config;
pub use server::config::{
    WarpgrapherConfig, WarpgrapherEndpoint, WarpgrapherResolverFunc, WarpgrapherResolvers,
    WarpgrapherType, WarpgrapherValidatorFunc, WarpgrapherValidators,
};
pub use server::context::{GraphQLContext, WarpgrapherRequestContext};
pub use server::extensions::{Extension, WarpgrapherExtensions};
pub use server::neo4j::Neo4jEndpoint;
pub use server::objects;
pub use server::objects::Node;
pub use server::schema::{Info, Property};
pub use server::Server;

pub mod client;
pub mod error;
pub mod server;
