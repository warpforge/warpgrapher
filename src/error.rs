//! This module provides error handling for WarpGrapher.
use serde_yaml;
use std::error;
use std::fmt::{Display, Formatter, Result};
use std::sync::mpsc::RecvError;

/// Categories of Warpgrapher errors.
#[derive(Debug)]
pub enum ErrorKind {
    /// Returned when the server attempts to listen on an address/port
    /// combination that is already bound on the system.
    AddrInUse(std::io::Error),

    /// Returned when the server attempts to listen on an address not
    /// assigned to any of the system's interfaces.
    AddrNotAvailable(std::io::Error),

    /// Returned when `WarpgrapherClient` receives an HTTP response which
    /// contains a body that is not valid JSON. All GraphQL responses
    /// including errors are expected to be in the form of valid JSON.
    ClientReceivedInvalidJson,

    /// Returned when `WarpgrapherClient` is unable to submit a request to
    /// the server (network error or server error).
    ClientRequestFailed,

    /// Returned when `WarpgrapherClient` receives a valid JSON response
    /// that does not contain the expected 'data' or 'errors' objects.
    ClientRequestUnexpectedPayload(serde_json::Value),

    /// Returned when a custom endpoint defines an inline custom input type
    /// with a name that conflicts with a GraphQL scalar
    ConfigEndpointInputTypeScalarNameError(String, String),

    /// Returned when a custom endpoint defines an inline custom output type
    /// with a name that conflicts with a GraphQL scalar
    ConfigEndpointOutputTypeScalarNameError(String, String),

    /// Returned when a warpgrapher type is defined with a name that conflicts with
    /// a GraphQL scalar
    ConfigTypeScalarNameError(String, String),

    /// Returned when a `Config` struct attempts to be initialized
    /// from a config file that cannot be found on disk.  
    ConfigNotFound(std::io::Error),

    /// Returned when a `Config` fails to deserialize because the
    /// provided data does not match the expected config spec
    ConfigDeserializationError(serde_yaml::Error),

    /// Returned when attempting to compose configs with different versions
    ConfigVersionMismatchError(String, i32),

    /// Returned when two warpgrapher types are defined with the same name
    ConfigTypeDuplicateError(String, String),

    /// Returned when two warpgrapher endpoints are defined with the same name
    ConfigEndpointDuplicateError(String, String),

    /// Returned when a warpgrapher endpoint defines for an input or output a
    /// type that does not exist
    ConfigEndpointMissingTypeError(String, String),

    /// Returned when `WarpgrapherServer` fails to build a pool for the cypher
    /// connection manager.
    CouldNotBuildCypherPool(r2d2::Error),

    /// Returned when the internal resolver logic cannot infer the correct warpgrapher type
    /// that corresponds to data queried from the database.
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    CouldNotInferType,

    /// Returned when an environment variable cannot be found
    EnvironmentVariableNotFound(String),

    /// Returned when there is a mismatch in the expected internal representation of a
    /// warpgrapher type
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    InputTypeMismatch(String),

    /// Returned when trying to perform on operation on a type that cannot support it.
    /// For example, this would be returned when trying to load a relationship from
    /// an input, as input types don't have relationships. This is a critical internal
    /// error. If you see it, please report it to the warpgrapher team.
    InvalidType(String),

    /// Returned when received GraphQL input contains an invalid property
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    InvalidProperty(String),

    /// Returned when there is a mismatch in the expected internal representation of a
    /// warpgrapher type
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    InvalidPropertyType(String),

    /// Returned during config validation if the config defines a Type or Rel property with the name 'ID'.
    /// ID is a reserved prop used by the Warpgrapher internals.
    InvalidPropNameID(String),

    /// Returned when attempts to serialize/deserialize a struct to/from JSON fails
    JsonError(serde_json::error::Error),

    /// Returned when attempts to convert a serde_json object to/from a String fails
    JsonStringConversionFailed(serde_json::error::Error),

    /// Returned when a resolver's input is missing an expected argument. Given
    /// GraphQL's type system
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    MissingArgument(String),

    /// Returned when warpgrapher missing a property expected for that node or rel type.
    /// This could occur if a node or relationship is created with a direct cypher query
    /// in a custom resolver or external to Warpgrapher altogether, without creating an
    /// 'id' property with a UUID. This could also occur if a schema change makes a
    /// previously optional and nullable property mandatory and non-nullable.
    MissingProperty(String, Option<String>),

    /// Returned when the cursor points outside of the bounds of the data returned
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    MissingResultSet,

    /// Returned when there is a mismatch between the data returned from the database
    /// and what the internal representation of a warpgrapher type expects
    MissingResultElement(String),

    /// Returned at start time when warpgrapher is dynamically generating a GraphQL schema
    /// from the config but there is a mismatch in the schema.
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    MissingSchemaElement(String),

    /// Returned when a field (prop or rel) of a node has been determined to be a DynamicScalar
    /// type and will attempt to execute a custom resolver for that field. This error is
    /// returned if the resolver is not defined for that DynamicScalar type field.
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    FieldMissingResolverError(String, String),

    /// Returned when there is a failure executing a neo4j query and the expected results
    /// from the database are not returned.
    GraphQueryError(rusted_cypher::error::GraphError),

    /// Returned when the output of a GraphQL execution is not a valid JSON.
    /// Note: This error should never be thrown. This is a critical error. If you see it,
    /// please report it to the warpgrapher team.
    GraphQLOutputError(String),

    /// Returned when a registered Pre Request Hook extension function returns an error
    PreRequestHookExtensionError(Box<dyn std::error::Error + Send + Sync>),

    /// Returned when a registered Post Requst Hook extension function returns an error
    PostRequestHookExtensionError(Box<dyn std::error::Error + Send + Sync>),

    /// Returned when a resolver attempt to infer relationships between queried data via
    /// a regex match fails
    RegexError,

    /// Returned when a custom endpoint is defined or a resolver is
    /// defined for a field, but the corresponding resolver is not provided.
    ResolverNotFound(String, String),

    /// Returned when a `WarpgrapherServer` tries to shutdown but the server is not
    /// running.
    ServerNotRunning,

    /// Returned when an error is encountered while trying to shutdown a `WarpgrapherServer`
    /// that is supposed to be running.
    ServerShutdownFailed,

    /// Returned when a `WarpgrapherServer` that is already running tries to start.
    ServerAlreadyRunning,

    /// Returned when a `WarpgrapherServer` fails to start.
    ServerStartupFailed(RecvError),

    /// Returned when a custom input validator is defined, but the corresponding
    /// validator is not provided.
    ValidatorNotFound(String, String),

    /// This error is returned by a custom input validator when the validation fails.
    /// This error is converted into a FieldError and returned to the client.
    ValidationError(String),
}

/// Error type for Warpgrapher operations.
///
/// Many errors originate in underlying libraries, but the
/// [`ErrorKind`] wraps these as necessary into
///  Warpgrapher errors.
///
/// [`ErrorKind`]: ./enum.ErrorKind.html
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    source: Option<Box<dyn error::Error + Send + Sync>>,
}

impl Error {
    /// Creates a new Warpgrapher error from an [`ErrorKind`] and,
    /// optionally, an arbitrary source error.
    ///
    /// [`ErrorKind`]: ./enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::{Error, ErrorKind};
    ///
    /// let e1 = Error::new(ErrorKind::ServerAlreadyRunning, None);
    ///
    /// let s = std::io::Error::new(std::io::ErrorKind::Other, "Oh no!");
    /// let e2 = Error::new(ErrorKind::ServerShutdownFailed, Some(Box::new(s)));
    /// ```
    pub fn new(kind: ErrorKind, source: Option<Box<dyn error::Error + Send + Sync>>) -> Error {
        Error { kind, source }
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        write!(
            fmt,
            "ErrorKind: {:#?}, source: {:#?}",
            self.kind, self.source
        )
    }
}

impl error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::{Error, ErrorKind};

    /// Passes if a new error with no wrapped source error is created
    #[test]
    fn new_error() {
        let e = Error::new(ErrorKind::ServerAlreadyRunning, None);

        assert!(std::error::Error::source(&e).is_none());
    }

    /// Passes if an error prints a display string correctly
    #[test]
    fn display_fmt() {
        let s = std::io::Error::new(std::io::ErrorKind::Other, "oh no!");
        let e = Error::new(ErrorKind::ServerShutdownFailed, Some(Box::new(s)));

        assert!(&format!("{}", e).starts_with("ErrorKind: ServerShutdownFailed, source: Some"));
    }
}
