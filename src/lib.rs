//! Warpgrapher is a framework for developing graph-based API services. Describe the data model for
//! which you want to run a web service.  Wargrapher automatically generates a GraphQL schema from
//! the data model, as well as a set of resolvers for basic create, read, update, and delete (CRUD)
//! operations on that data. If you need more more sophisticated endpoints, you
//! can supply your own custom resolvers. Warpgrapher will automatically
//! generate the GraphQL configuration and invoke your custom resolvers when
//! appropriate.
//!
//! For an introduction and tutorials, see the [Warpgrapher Book](https://warpforge.github.io/warpgrapher/).
//!
//! Warpgrapher is published as [Cargo Crate](https://crates.io/crates/warpgrapher).
//!
//! To browse source code, report issues, or contribute to the project, see the [GitHub Repository](https://github.com/warpforge/warpgrapher).

#![doc(html_root_url = "https://docs.rs/warpgrapher/0.10.4")]

#[cfg(feature = "cypher")]
pub use bolt_client;
#[cfg(feature = "cypher")]
pub use bolt_proto;
#[cfg(feature = "gremlin")]
pub use gremlin_client;
pub use juniper;

pub use client::Client;
pub use engine::config::Configuration;
pub use engine::database::DatabasePool;
pub use engine::value::Value;
pub use engine::Engine;
pub use error::Error;

pub mod client;
pub mod engine;
mod error;
