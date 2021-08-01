//! Provides the [`Error`] type for Warpgrapher

#[cfg(feature = "gremlin")]
use gremlin_client::GremlinError;
use http::header::{InvalidHeaderName, InvalidHeaderValue};
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::ParseBoolError;

/// Error type for Warpgrapher
///
/// # Examples
///
/// ```rust
/// use serde_json::json;
/// use warpgrapher::Error;
///
/// let e = Error::PayloadNotFound { response: json!{"surprise"} };
/// ```
#[derive(Debug)]
pub enum Error {
    /// Returned to wrap an error from the Neo4J bolt client. Most likely indicates something like
    /// a network connection failure
    #[cfg(feature = "neo4j")]
    BoltClientFailed {
        source: bolt_client::error::Error,
    },

    /// Returned if a [`Client`] is unable to submit a request to the server, such as due to a
    /// network or server error, or the response cannot be parsed as valid JSON. Inspect the
    /// [`reqwest::Error`] included as a source error for additional detail.
    ///
    /// [`Client`]: ./client/enum.Client.html
    ClientRequestFailed {
        source: reqwest::Error,
    },

    /// Returned if two Warpgrapher endpoints or two Warpgrapher types are defined with the same
    /// name. The `type_name` field contains the name of the duplicated type.
    ConfigItemDuplicated {
        type_name: String,
    },

    /// Returned if a Warpgrapher endpoint or type is defined with a name that is a reserved
    /// word, such as "ID" or a GraphQL scalar. The field `type_name` is the name that triggered the
    /// error.
    ConfigItemReserved {
        type_name: String,
    },

    /// Returned if a `Config` file cannot be opened, typically because the configuration file
    /// cannot be found on disk
    ConfigOpenFailed {
        source: std::io::Error,
    },

    /// Returned if attempting to compose configs with different versions. The field `expected`
    /// contains the version of the base `Config`, and `found` contains the version of the `Config`
    /// being merged in.
    ///
    /// [`Config`]: ../engine/config/struct.Config.html
    ConfigVersionMismatched {
        expected: i32,
        found: i32,
    },

    /// Returned if the engine is configured to operate without a database. Typically this would
    /// never be done in production
    DatabaseNotFound,

    /// Returned if a `serde_json::Value` struct fails to deserialize into a struct
    JsonDeserializationFailed {
        source: serde_json::Error,
    },

    /// Returned if an environment variable cannot be found. The `name` field contains the name of
    /// the environment variable that could not be found.
    EnvironmentVariableNotFound {
        name: String,
    },

    /// Returned if an environemtn variable for a boolean flag cannot be parsed from the
    /// environment variable string into a bool
    EnvironmentVariableBoolNotParsed {
        source: ParseBoolError,
    },

    /// Returned if an environment variable for a port number cannot be parsed from the
    /// environment variable string into a number
    EnvironmentVariableIntNotParsed {
        source: ParseIntError,
    },

    /// Returned if a registered extension function returns an error
    EventError {
        source: Box<dyn std::error::Error + Sync + Send>,
    },

    /// Returned if a client for a Cosmos or Gremlin database pool cannot be built or a query fails.
    #[cfg(feature = "gremlin")]
    GremlinActionFailed {
        source: Box<gremlin_client::GremlinError>,
    },

    /// Returned if a GraphQL query is missing an expected argument. For example, if a create
    /// mutation call were missing its input argument. Also returned if an input argument is
    /// missing an expected field.
    InputItemNotFound {
        name: String,
    },

    /// Returned if an invalid header is passed to the constructor for creating an http client.
    /// There are two possible invalid header types based on whether the name or value was invalid.
    InvalidHeaderName {
        source: InvalidHeaderName,
    },
    InvalidHeaderValue {
        source: InvalidHeaderValue,
    },

    /// Returned if an internal CRUD handler tries to retrieve the label for a node or relationship
    /// from an internal temporary bookkeeping structure and is unable to do so. This almost
    /// certainly indicates an internal bug. Thus, if you happen to see it, please open an issue at
    /// the Warpgrapher project.
    LabelNotFound,

    /// Returned if a Neo4J pool cannot return a client transaction. The source error contains
    /// additional information.
    #[cfg(feature = "neo4j")]
    Neo4jPoolFailed {
        source: mobc::Error<mobc_boltrs::Error>,
    },

    /// Returned if a Neo4J pool cannot be built, such as if the host, port, or authentication
    /// credentials are incorrect. The source error contains additional information.
    #[cfg(feature = "neo4j")]
    Neo4jPoolNotBuilt {
        source: mobc_boltrs::Error,
    },

    /// Returned if a Neo4J query fails to execute correctly
    #[cfg(feature = "neo4j")]
    Neo4jQueryFailed {
        message: bolt_proto::message::Message,
    },

    /// Returned if a partition key is [`None`] for a database back-end that requires one, such as
    /// Cosmos DB
    PartitionKeyNotFound,

    /// Returned if a [`Client`] receives a valid JSON response that does not contain the
    /// expected 'data' or 'errors' objects.
    ///
    /// The [`serde_json::Value`] tuple value contains the deserialized JSON response.
    ///
    /// [`Client`]: ./client/enum.Client.html
    PayloadNotFound {
        response: serde_json::Value,
    },

    /// Return if a query tries to read and return a relationship defined in the GraphQL schema as
    /// being a single relationship (one-to-one), for which the back-end database has multiple
    /// outgoing relationship edges (one-to-many or many-to-many).  The `rel_name` field holds the
    /// name of the relationship, and the `ids` field holds a list of ids of the relationships
    /// found.
    RelDuplicated {
        rel_name: String,
        ids: String,
    },

    /// Returned if a custom endpoint is defined or a resolver is defined for a field, but the
    /// corresponding resolver is not provided. The `name` field contains the name of the resolver
    /// that could not be found.
    ResolverNotFound {
        name: String,
    },

    /// Returned if a database query is missing a set of results altogether, where one is expected.
    /// This likely indicates an internal bug. Thus, if you happen to see it, please open an issue
    /// at the Warpgrapher project.
    ResponseSetNotFound,

    /// Returned if a database query result is missing an expected property. For example, if a
    /// Cosmos DB query were to be missing the value for a property, or if the query fails to
    /// to return an expected node or relationship. This could occur if a custom resolver creates a
    /// node or rel witout adding mandatory properties, such as an ID.
    ResponseItemNotFound {
        name: String,
    },

    /// Returned if a GraphQL response or a database query parameter cannot be converted to a
    /// serde_json::Value, or if a query
    SerializationFailed {
        source: serde_json::Error,
    },

    /// Returned if Warpgrapher fails to find an element within a schema, such as a type or
    /// property. This is very unlikely to be returned as a result of problems with inputs to the
    /// engine and most likely indicates an internal bug. Thus, if you happen to see it, please
    /// open an issue at the Warpgrapher project.  The field is the name of the schema element that
    /// could not be fiound.
    SchemaItemNotFound {
        name: String,
    },

    /// When the Warpgrapher client sends queries to a local instance of a Warpgrapher engine,
    /// it runs the engine in a separate thread, where it can have its own tokio execution context.
    /// This error indicates an error in receiving the query answer from the engine thread.
    ThreadCommunicationFailed {
        source: std::sync::mpsc::RecvError,
    },

    /// Returned if a transaction is used after it is committed or rolled back.
    TransactionFinished,

    /// Warpgrapher transforms data between different serialization formats in the course of
    /// relaying data between GraphQL and database back-ends. If data fails to convert successfully,
    /// this error is thrown. The `src` field contains the source type name or value that could not
    /// be converted.
    TypeConversionFailed {
        src: String,
        dst: String,
    },

    /// Returned in multiple circumstances if the type information associated with a [`Value`] is
    /// inconsistent with the type required. Examples include:
    ///
    /// * a Warpgrapher [`Value`] enum doesn't match the variant expected for a given property, such
    /// as an ID represented by something other than a string value
    /// * a configuration schema is parsed incorrectly, resulting in an unexpected GraphQL endpoint
    /// input argument
    ///
    /// This error could be returned if a custom resolver creates a node or relationship with a
    /// property of a type that doesn't match the type in the schema, such a creating an integer
    /// property where the schema configures Warpgrapher to expect a string. However, other than
    /// that case, this error most likely indicates an internal bug for which an issue should be
    /// opened at the Warpgrapher project.
    ///
    /// [`Value`]: ./engine/value/enum.Value.html
    TypeNotExpected {
        details: Option<String>,
    },

    /// Returned when encapsulating an error thrown in event handlers provided by users of
    /// Warpgrapher
    UserDefinedError {
        source: Box<dyn std::error::Error + Sync + Send>,
    },

    /// Returned if the String argument for an id cannot be parsed into a UUID
    UuidNotParsed {
        source: uuid::Error,
    },

    /// This error is returned by a custom input validator when the validation fails. The message
    /// String describes the reason the field failed validation.
    ValidationFailed {
        message: String,
    },

    /// Returned if a custom input validator is defined, but the corresponding validator is not
    /// provided. The `name` field contains the name of the validator that wasn't found.
    ValidatorNotFound {
        name: String,
    },

    /// Returned if a `serde_yaml::Value` struct fails to deserialize into a given struct
    YamlDeserializationFailed {
        source: serde_yaml::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            #[cfg(feature = "neo4j")]
            Error::BoltClientFailed { source } => {
                write!(f, "Neo4j client failed. Source error: {}.", source)
            }
            Error::ClientRequestFailed { source } => {
                write!(f, "Client request failed. Source error: {}", source)
            }
            Error::ConfigItemDuplicated { type_name } => {
                write!(f, "Config model contains duplicate item: {}", type_name)
            }
            Error::ConfigItemReserved { type_name } => {
                write!(
                    f,
                    "Config item cannot use a reserved word as a name: {}",
                    type_name
                )
            }
            Error::ConfigOpenFailed { source } => {
                write!(
                    f,
                    "Config file could not be opened. Source error: {}",
                    source
                )
            }
            Error::ConfigVersionMismatched { expected, found } => {
                write!(
                    f,
                    "Configs must be the same version: expected {} but found {}",
                    expected, found
                )
            }
            Error::DatabaseNotFound => {
                write!(f, "Use of resolvers required a database back-end. Please select either cosmos or neo4j.")
            }
            Error::EnvironmentVariableNotFound { name } => {
                write!(f, "Could not find environment variable: {}", name)
            }
            Error::EnvironmentVariableBoolNotParsed { source } => {
                write!(
                    f,
                    "Failed to parse environment variable to boolean flag. Source error: {}",
                    source
                )
            }
            Error::EnvironmentVariableIntNotParsed { source } => {
                write!(
                    f,
                    "Failed to parse environment variable to integer port number. Source error: {}",
                    source
                )
            }
            Error::EventError { source } => {
                write!(f, "Event handler returned an error: {}", source)
            }
            #[cfg(feature = "gremlin")]
            Error::GremlinActionFailed { source } => {
                write!(
                    f,
                    "Either building a database connection pool or query failed. Source error: {}",
                    source
                )
            }
            Error::InputItemNotFound { name } => {
                write!(
                    f,
                    "Could not find an expected argument, {}, in the GraphQL query.",
                    name
                )
            }
            Error::InvalidHeaderName { source } => {
                write!(f, "Invalid HTTP header given to Client: {}", source)
            }
            Error::InvalidHeaderValue { source } => {
                write!(f, "Invalid HTTP header given to Client: {}", source)
            }
            Error::LabelNotFound => {
                write!(f, "Could not find a label for a destination node.")
            }
            Error::JsonDeserializationFailed { source } => {
                write!(f, "Failed to deserialize JSON into struct: {}", source)
            }
            #[cfg(feature = "neo4j")]
            Error::Neo4jPoolFailed { source } => {
                write!(f, "Could not get Neo4J transaction from pool: {}", source)
            }
            #[cfg(feature = "neo4j")]
            Error::Neo4jPoolNotBuilt { source } => {
                write!(f, "Could not build Neo4j connection pool: {}", source)
            }
            #[cfg(feature = "neo4j")]
            Error::Neo4jQueryFailed { message } => {
                write!(
                    f,
                    "Neo4j query execution failed. Error message: {:#?}.",
                    message
                )
            }
            Error::PartitionKeyNotFound => {
                write!(f, "Partition keys are required when using Cosmos DB.")
            }
            Error::PayloadNotFound { response } => {
                write!(
                    f,
                    "Required data and/or error fields are missing from the response: {}",
                    response
                )
            }
            Error::RelDuplicated { rel_name, ids } => {
                write!(f, "Tried to read the single-node (i.e. one-to-one) relationship named {}, but found multipled ids: {}", rel_name, ids)
            }
            Error::ResolverNotFound { name } => {
                write!(f, "Could not find a custom resolver named {}", name)
            }
            Error::ResponseItemNotFound { name } => {
                write!(
                    f,
                    "Could not find an expected response item, {}, in the database results.",
                    name
                )
            }
            Error::ResponseSetNotFound => {
                write!(f, "Could not find an expected database set of results.")
            }
            Error::SerializationFailed { source } => {
                write!(
                    f,
                    "Serialization of the GraphQL response failed. Source error: {}",
                    source
                )
            }
            Error::SchemaItemNotFound { name } => {
                write!(
                    f,
                    "The following item could not be found in the schema: {}",
                    name
                )
            }
            Error::ThreadCommunicationFailed { source } => {
                write!(
                    f,
                    "Communication from the engine thread failed. Source error: {}",
                    source
                )
            }
            Error::TransactionFinished => {
                write!(
                    f,
                    "Cannot use a database transaction already committed or rolled back."
                )
            }
            Error::TypeConversionFailed { src, dst } => {
                write!(
                    f,
                    "The type or value {} could not be converted to type {}",
                    src, dst
                )
            }
            Error::TypeNotExpected { details } => {
                write!(
                    f,
                    "Warpgrapher encountered a type that was not expected {}",
                    if let Some(s) = details {
                        format!("({:#?}", s)
                    } else {
                        "".to_string()
                    }
                )
            }
            Error::UserDefinedError { source } => {
                write!(f, "User defined error. Source error: {:#?}", source)
            }
            Error::UuidNotParsed { source } => {
                write!(
                    f,
                    "Failed to parse id attribute value. Source error: {}",
                    source
                )
            }
            Error::ValidationFailed { message } => {
                write!(f, "{}", message)
            }
            Error::ValidatorNotFound { name } => {
                write!(f, "A validator function named {} could not be found", name)
            }
            Error::YamlDeserializationFailed { source } => {
                write!(
                    f,
                    "Failed to deserialize yaml struct. Source error: {}",
                    source
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "neo4j")]
            Error::BoltClientFailed { source } => Some(source),
            Error::ClientRequestFailed { source } => Some(source),
            Error::ConfigItemDuplicated { type_name: _ } => None,
            Error::ConfigItemReserved { type_name: _ } => None,
            Error::ConfigOpenFailed { source } => Some(source),
            Error::ConfigVersionMismatched {
                expected: _,
                found: _,
            } => None,
            Error::DatabaseNotFound => None,
            Error::EnvironmentVariableNotFound { name: _ } => None,
            Error::EnvironmentVariableBoolNotParsed { source } => Some(source),
            Error::EnvironmentVariableIntNotParsed { source } => Some(source),
            Error::EventError { source } => Some(source.as_ref()),
            #[cfg(feature = "gremlin")]
            Error::GremlinActionFailed { source } => Some(source),
            Error::InputItemNotFound { name: _ } => None,
            Error::InvalidHeaderName { source } => Some(source),
            Error::InvalidHeaderValue { source } => Some(source),
            Error::JsonDeserializationFailed { source } => Some(source),
            Error::LabelNotFound => None,
            #[cfg(feature = "neo4j")]
            Error::Neo4jPoolFailed { source } => Some(source),
            #[cfg(feature = "neo4j")]
            Error::Neo4jPoolNotBuilt { source } => Some(source),
            #[cfg(feature = "neo4j")]
            Error::Neo4jQueryFailed { message: _ } => None,
            Error::PartitionKeyNotFound => None,
            Error::PayloadNotFound { response: _ } => None,
            Error::RelDuplicated {
                rel_name: _,
                ids: _,
            } => None,
            Error::ResolverNotFound { name: _ } => None,
            Error::ResponseItemNotFound { name: _ } => None,
            Error::ResponseSetNotFound => None,
            Error::SerializationFailed { source } => Some(source),
            Error::SchemaItemNotFound { name: _ } => None,
            Error::ThreadCommunicationFailed { source } => Some(source),
            Error::TransactionFinished => None,
            Error::TypeConversionFailed { src: _, dst: _ } => None,
            Error::TypeNotExpected { details: _ } => None,
            Error::UserDefinedError { source: _ } => None,
            Error::UuidNotParsed { source } => Some(source),
            Error::ValidationFailed { message: _ } => None,
            Error::ValidatorNotFound { name: _ } => None,
            Error::YamlDeserializationFailed { source } => Some(source),
        }
    }
}

impl From<Box<dyn std::error::Error + Sync + Send>> for Error {
    fn from(e: Box<dyn std::error::Error + Sync + Send>) -> Self {
        Error::EventError { source: e }
    }
}

#[cfg(feature = "neo4j")]
impl From<bolt_client::error::Error> for Error {
    fn from(e: bolt_client::error::Error) -> Self {
        Error::BoltClientFailed { source: e }
    }
}

#[cfg(feature = "neo4j")]
impl From<bolt_proto::error::Error> for Error {
    fn from(_e: bolt_proto::error::Error) -> Self {
        Error::TypeConversionFailed {
            src: "bolt_proto::value::Value".to_string(),
            dst: "Value".to_string(),
        }
    }
}

#[cfg(feature = "gremlin")]
impl From<GremlinError> for Error {
    fn from(e: GremlinError) -> Self {
        Error::GremlinActionFailed {
            source: Box::new(e),
        }
    }
}

#[cfg(feature = "neo4j")]
impl From<mobc::Error<mobc_boltrs::Error>> for Error {
    fn from(e: mobc::Error<mobc_boltrs::Error>) -> Self {
        Error::Neo4jPoolFailed { source: e }
    }
}

#[cfg(feature = "neo4j")]
impl From<mobc_boltrs::Error> for Error {
    fn from(e: mobc_boltrs::Error) -> Self {
        Error::Neo4jPoolNotBuilt { source: e }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::ClientRequestFailed { source: e }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerializationFailed { source: e }
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::YamlDeserializationFailed { source: e }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::ConfigOpenFailed { source: e }
    }
}

impl From<std::str::ParseBoolError> for Error {
    fn from(e: std::str::ParseBoolError) -> Self {
        Error::EnvironmentVariableBoolNotParsed { source: e }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::EnvironmentVariableIntNotParsed { source: e }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_e: std::num::TryFromIntError) -> Self {
        Error::TypeConversionFailed {
            src: "i64 or uint64".to_string(),
            dst: "i32".to_string(),
        }
    }
}

impl From<std::sync::mpsc::RecvError> for Error {
    fn from(e: std::sync::mpsc::RecvError) -> Self {
        Error::ThreadCommunicationFailed { source: e }
    }
}

impl From<uuid::Error> for Error {
    fn from(e: uuid::Error) -> Self {
        Error::UuidNotParsed { source: e }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    /// Passes if a new error with no wrapped source error is created
    #[test]
    fn new_error() {
        let e = Error::DatabaseNotFound;

        assert!(std::error::Error::source(&e).is_none());
    }

    /// Passes if an error prints a display string correctly
    #[test]
    fn display_fmt() {
        let s = std::io::Error::new(std::io::ErrorKind::Other, "oh no!");
        let e = Error::ConfigOpenFailed { source: s };

        assert_eq!(
            "Config file could not be opened. Source error: oh no!",
            &format!("{}", e)
        );
    }

    /// Passes if Error implements the Send trait
    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    /// Passes if Client implements the Sync trait
    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Error>();
    }
}
