//! Warpgrapher makes it painless to create web services with graph-based data
//! models. Describe the data model for which you want to run a web service.
//! Wargrapher automatically generates a GraphQL schema from the data model, as
//! well as a set of resolvers for basic create, read, update, and delete (CRUD)
//! operations on that data. If you need more more sophisticated endpoints, you
//! can supply your own custom resolvers. Warpgrapher will automatically
//! generate the GraphQL configuration and invoke your custom resolvers when
//! appropriate.

#[macro_use]
extern crate juniper;

pub use juniper::{Arguments, ExecutionResult, Executor, Object, Value};

pub use client::WarpgrapherClient;
pub use error::Error;
pub use error::ErrorKind;

pub use engine::config::{
    WarpgrapherConfig, WarpgrapherEndpoint, WarpgrapherResolverFunc, WarpgrapherResolvers,
    WarpgrapherType, WarpgrapherValidatorFunc, WarpgrapherValidators,
};
pub use engine::context::{GraphQLContext, WarpgrapherRequestContext};
pub use engine::extensions::{Extension, WarpgrapherExtensions};
pub use engine::neo4j::Neo4jEndpoint;
pub use engine::objects::Node;
pub use engine::schema::{Info, Property};
pub use engine::Engine;

pub mod client;
pub mod engine;
pub mod error;
