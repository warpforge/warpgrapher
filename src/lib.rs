// t0
//! Warpgrapher makes it painless to create web services with graph-based data
//! models. Describe the data model for which you want to run a web service.
//! Wargrapher automatically generates a GraphQL schema from the data model, as
//! well as a set of resolvers for basic create, read, update, and delete (CRUD)
//! operations on that data.  If you need more more sophisticated endpoints, you
//! can supply your own custom resolvers. Warpgrapher will automatically
//! generate the GraphQL configuration and invoke your custom resolvers when
//! appropriate.
//!
//! * [Cargo Crate](https://crates.io/crates/warpgrapher)
//! * [Warpgrapher Book](https://warpforge.github.io/warpgrapher/)

#![doc(html_root_url = "https://docs.rs/warpgrapher/0.2.0")]

pub use juniper::http::playground::playground_source;
pub use juniper::http::GraphQLRequest;
pub use juniper::FieldError;

pub use client::Client;
pub use engine::config::Configuration;
pub use engine::database::DatabasePool;
pub use engine::Engine;
pub use error::Error;

pub mod client;
pub mod engine;
mod error;
