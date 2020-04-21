//! models and custom GraphQL endpoints.

//use std::fmt;
use super::context::GraphQLContext;
use super::schema::Info;
use crate::error::{Error, ErrorKind};
use juniper::{Arguments, ExecutionResult, Executor};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

/// Convenience function for setting serde default value
fn get_false() -> bool {
    false
}

/// Convenience function for setting serde default value
fn get_true() -> bool {
    true
}

fn get_none() -> Option<String> {
    None
}

pub type WarpgrapherResolverFunc<GlobalCtx, ReqCtx> =
    fn(&Info, &Arguments, &Executor<GraphQLContext<GlobalCtx, ReqCtx>>) -> ExecutionResult;

pub type WarpgrapherResolvers<GlobalCtx, ReqCtx> =
    HashMap<String, Box<WarpgrapherResolverFunc<GlobalCtx, ReqCtx>>>;

pub type WarpgrapherValidatorFunc = fn(&Value) -> Result<(), Error>;

pub type WarpgrapherValidators = HashMap<String, Box<WarpgrapherValidatorFunc>>;

//pub enum WarpgrapherPropType {
//}

/// Configuration item for a Warpgrapher data model. The configuration contains
/// the version of the Warpgrapher configuration file format, and a vector of
/// [`WarpgrapherType`] structures.
///
/// [`WarpgrapherType`]: struct.WarpgrapherType.html
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::Config;
///
/// let c = Config::new(1, Vec::new(), Vec::new());
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Version of the Warpgrapher configuration file format used
    pub version: i32,

    /// A vector of [`WarpgrapherType`] structures, each defining one type in
    /// the data model
    ///
    /// [`WarpgrapherType`]: struct.WarpgrapherType.html
    #[serde(default)]
    pub model: Vec<WarpgrapherType>,

    /// A vector of [`Endpoint`] structures, each defining an
    /// a custom root endpoint in the graphql schema
    ///
    /// [`Endpoint`]: struct.Endpoint.html
    #[serde(default)]
    pub endpoints: Vec<Endpoint>,
}

impl Config {
    /// Creates a new, empty [`Config`] data structure
    ///
    /// [`Config`]: struct.Config.html
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::Config;
    ///
    /// let c = Config::new(1, Vec::new(), Vec::new());
    /// ```
    pub fn new(
        version: i32,
        model: Vec<WarpgrapherType>,
        endpoints: Vec<Endpoint>,
    ) -> Config {
        Config {
            version,
            model,
            endpoints,
        }
    }

    /// Creates a new, [`Config`] data structure with default
    /// values
    ///
    /// [`Config`]: struct.Config.html
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::Config;
    ///
    /// let config = Config::default();
    /// ```
    pub fn default() -> Config {
        Config {
            version: 1,
            model: vec![],
            endpoints: vec![],
        }
    }

    /// Creates a new [`Config`] data structure from
    /// the contents of the specified config file. Returns error
    /// if the config file could not be opened or deserialized.
    pub fn from_file(path: String) -> Result<Config, Error> {
        File::open(path)
            .map_err(|e| Error::new(ErrorKind::ConfigNotFound(e), None))
            .and_then(|f| {
                let r = BufReader::new(f);
                serde_yaml::from_reader(r)
                    .map_err(|e| Error::new(ErrorKind::ConfigDeserializationError(e), None))
            })
    }

    /// Validates the [`Config`] data structure.
    /// Checks to verify no duplicate [`Endpoint`] or [`WarpgrapherType`], and that the
    /// [`Endpoint`] input/output types are defined in the model.
    /// Returns a Result<(), Error> where the error could be one of:
    /// - [`ConfigTypeDuplicateError`] if any Type is defined twice in the configuration.
    /// - [`ConfigEndpointDuplicateError`] if any Endpoint Type is defined twice in the configuration.
    /// - [`ConfigEndpointMissingTypeError`] if an Endpoint does not have a corresponding Type defined.
    ///
    /// #Example
    /// ```rust
    ///     use warpgrapher::engine::config::{Config};
    ///
    ///     let config = Config::new(1, Vec::new(), Vec::new());
    ///     config.validate();
    /// ```

    pub fn validate(&self) -> Result<(), Error> {
        let scalar_names = ["Int", "Float", "Boolean", "String", "ID"];

        for t in &self.model {
            // Check for duplicate types
            if self.model.iter().filter(|m| m.name == t.name).count() > 1 {
                return Err(Error::new(
                    ErrorKind::ConfigTypeDuplicateError(
                        format!(
                            "Config Model contains duplicate Type: {type_name}.",
                            type_name = t.name
                        ),
                        t.name.clone(),
                    ),
                    None,
                ));
            }

            // Check for types using reserved names (GraphQL scalars)
            if scalar_names.iter().any(|s| s == &t.name) {
                return Err(Error::new(
                    ErrorKind::ConfigTypeScalarNameError(
                        format!(
                            "Type cannot have the name of a scalar type: {type_name}.",
                            type_name = t.name
                        ),
                        t.name.clone(),
                    ),
                    None,
                ));
            }

            if t.props.iter().any(|p| p.name == "ID") {
                return Err(Error::new(
                    ErrorKind::InvalidPropNameID("Prop cannot have the name ID.".to_string()),
                    None,
                ));
            }

            for r in t.rels.iter() {
                if r.props.iter().any(|p| p.name == "ID") {
                    return Err(Error::new(
                        ErrorKind::InvalidPropNameID("Prop cannot have the name ID.".to_string()),
                        None,
                    ));
                }
            }
        }

        for ep in &self.endpoints {
            // Check for duplicate endpoints
            if self.endpoints.iter().filter(|e| e.name == ep.name).count() > 1 {
                return Err(Error::new(
                    ErrorKind::ConfigEndpointDuplicateError(
                        format!(
                            "Config contains duplicate Endpoints: {endpoint_name}.",
                            endpoint_name = ep.name
                        ),
                        ep.name.clone(),
                    ),
                    None,
                ));
            }

            // Check for endpoint custom input using reserved names (GraphQL scalars)
            if let Some(input) = &ep.input {
                if let WarpgrapherTypeDef::Custom(t) = &input.type_def {
                    if scalar_names.iter().any(|s| s == &t.name) {
                        return Err(Error::new(
                                ErrorKind::ConfigEndpointInputTypeScalarNameError(
                                    format!(
                                        "Endpoint Input Type cannot have the name of a scalar type: {type_name}.",
                                        type_name = t.name
                                    ),
                                    t.name.clone(),
                                ),
                                None,
                            ));
                    }
                }
            }

            // Check for endpoint custom input using reserved names (GraphQL scalars)
            if let WarpgrapherTypeDef::Custom(t) = &ep.output.type_def {
                if scalar_names.iter().any(|s| s == &t.name) {
                    return Err(Error::new(
                        ErrorKind::ConfigEndpointOutputTypeScalarNameError(
                            format!(
                                "Endpoint Output Type cannot have the name of a scalar type: {type_name}.",
                                type_name = t.name
                            ),
                            t.name.clone(),
                        ),
                        None,
                    ));
                }
            }

            /*
            // TODO: need to move this inside engine::validate() since it is possible for an endpoint
            // input to be an auto-generated type which cannot be introspected from the context of the
            // config alone
            match &ep.input.type_def {
                WarpgrapherTypeDef::Null => { }
                WarpgrapherTypeDef::Scalar(_) => { }
                WarpgrapherTypeDef::Existing(t) => {
                    if !self.model.iter().any(|m| &m.name == t) {
                        return Err(Error::new(
                            ErrorKind::ConfigEndpointMissingTypeError(
                                format!(
                                    "Endpoint Input Type is not defined: {type_name}.",
                                    type_name = t
                                ),
                                t.clone(),
                            ),
                            None,
                        ));
                    }
                }
                WarpgrapherTypeDef::Custom(_) => { }
                WarpgrapherTypeDef::Custom(t) => {
                    if !self.model.iter().any(|m| m.name == t.name) {
                        return Err(Error::new(
                            ErrorKind::ConfigEndpointMissingTypeError(
                                format!(
                                    "Endpoint Input Type is not defined: {type_name}.",
                                    type_name = t.name
                                ),
                                t.name.clone(),
                            ),
                            None,
                        ));
                    }
                }
            }

            // TODO: need to move this inside engine::validate() since it is possible for an endpoint
            // input to be an auto-generated type which cannot be introspected from the context of the
            // config alone
            match &ep.output.type_def {
                WarpgrapherTypeDef::Scalar(_) => {}
                WarpgrapherTypeDef::Existing(t) => {
                    if !self.model.iter().any(|m| &m.name == t) {
                        return Err(Error::new(
                            ErrorKind::ConfigEndpointMissingTypeError(
                                format!(
                                    "Endpoint Output Type is not defined: {type_name}.",
                                    type_name = t
                                ),
                                t.clone(),
                            ),
                            None,
                        ));
                    }
                }
                WarpgrapherTypeDef::Custom(t) => {
                    if !self.model.iter().any(|m| m.name == t.name) {
                        return Err(Error::new(
                            ErrorKind::ConfigEndpointMissingTypeError(
                                format!(
                                    "Endpoint Output Type is not defined: {type_name}.",
                                    type_name = t.name
                                ),
                                t.name.clone(),
                            ),
                            None,
                        ));
                    }
                }
            }
            */
        }

        Ok(())
    }
}

/// Configuration item for a property on a GraphQL type, modeled as properties
///  on a Neo4J node.
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::WarpgrapherProp;
///
/// let p = WarpgrapherProp::new("name".to_string(), "String".to_string(), true, false, None, None);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpgrapherProp {
    /// Name of the property
    pub name: String,

    /// The name of the type of the property (e.g. String)
    #[serde(rename = "type")]
    pub type_name: String,

    /// True if this property is required to be present on this type; false if
    /// the property is optional
    #[serde(default = "get_false")]
    pub required: bool,

    /// True if this property is a list
    #[serde(default = "get_false")]
    pub list: bool,

    /// The name of the resolver function to be called when querying for the value of this prop.
    /// If this field is None, the prop resolves the scalar value from the database.
    #[serde(default = "get_none")]
    pub resolver: Option<String>,

    /// The name of the validator function to be called when creating or modifying the value of
    /// this prop. If this field is None, the prop resolves the scalar value from the database.
    #[serde(default = "get_none")]
    pub validator: Option<String>,
}

impl WarpgrapherProp {
    /// Creates a new WarpgrapherProp struct. Takes a String for the name of
    /// the property, a String for the type of the property, and a boolean
    /// that, if true, indicates the property is required, and if false, that
    /// the property is optional.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::WarpgrapherProp;
    ///
    /// let p = WarpgrapherProp::new("name".to_string(), "String".to_string(), true, false, None, None);
    /// ```
    pub fn new(
        name: String,
        type_name: String,
        required: bool,
        list: bool,
        resolver: Option<String>,
        validator: Option<String>,
    ) -> WarpgrapherProp {
        WarpgrapherProp {
            name,
            type_name,
            required,
            list,
            resolver,
            validator,
        }
    }
}

/// Configuration item for a relationship on a GraphQL type
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::{WarpgrapherRel, EndpointsFilter};
///
/// let p = WarpgrapherRel::new(
///            "teams".to_string(),
///            true,
///            vec!["User".to_string()],
///            vec![],  
///            EndpointsFilter::all()
///         );
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpgrapherRel {
    /// Name of the relationship
    pub name: String,

    /// True if its a multi-node relationship
    #[serde(default = "get_false")]
    pub list: bool,

    /// List of possible dst nodes for the relationship. A single element
    /// vector indicates a single-type rel and more than one element
    /// indicates a multi-type relationship.
    pub nodes: Vec<String>,

    /// Properties of the relationship
    #[serde(default)]
    pub props: Vec<WarpgrapherProp>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    pub endpoints: EndpointsFilter,
}

impl WarpgrapherRel {
    /// Creates a new, empty [`WarpgrapherRel`] data structure
    pub fn new(
        name: String,
        list: bool,
        nodes: Vec<String>,
        props: Vec<WarpgrapherProp>,
        endpoints: EndpointsFilter,
    ) -> WarpgrapherRel {
        WarpgrapherRel {
            name,
            list,
            nodes,
            props,
            endpoints,
        }
    }
}

/// Configuration item for endpoint filters
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::{EndpointsFilter};
///
/// let ef = EndpointsFilter::new(true, true, true, true);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EndpointsFilter {
    /// True if a read endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    pub read: bool,

    /// True if a create endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    pub create: bool,

    /// True if a update endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    pub update: bool,

    /// True if a delete endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    pub delete: bool,
}

impl EndpointsFilter {
    /// Creates a new filter with the option to configure all endpoints
    pub fn new(read: bool, create: bool, update: bool, delete: bool) -> EndpointsFilter {
        EndpointsFilter {
            read,
            create,
            update,
            delete,
        }
    }

    /// Creates a new filter with all endpoints set to true
    pub fn all() -> EndpointsFilter {
        EndpointsFilter {
            read: true,
            create: true,
            update: true,
            delete: true,
        }
    }

    /// Creates a new filter with all endpoints set to false
    pub fn none() -> EndpointsFilter {
        EndpointsFilter {
            read: false,
            create: false,
            update: false,
            delete: false,
        }
    }
}

impl Default for EndpointsFilter {
    fn default() -> EndpointsFilter {
        EndpointsFilter {
            read: true,
            create: true,
            update: true,
            delete: true,
        }
    }
}

/// Configuration item for a GraphQL type, also represented as a Neo4J label
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::{WarpgrapherType, WarpgrapherProp, EndpointsFilter};
///
/// let wt = WarpgrapherType::new(
///     "User".to_string(),
///     vec!(WarpgrapherProp::new("name".to_string(), "String".to_string(), true, false, None, None),
///          WarpgrapherProp::new("role".to_string(), "String".to_string(), true, false, None, None)),
///     vec!(),
///     EndpointsFilter::all()
/// );
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpgrapherType {
    /// Name of this GraphQL type, also used as the Neo4J label for nodes
    pub name: String,

    /// Vector of properties on this type
    pub props: Vec<WarpgrapherProp>,

    /// Vector of relationships on this type
    #[serde(default)]
    pub rels: Vec<WarpgrapherRel>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    pub endpoints: EndpointsFilter,
}

impl WarpgrapherType {
    /// Creates a new WarpgrapherType struct. Takes a String name for the type
    /// and a vector of [`WarpgrapherProp`] structs and returns a
    /// [`WarpgrapherType`].
    ///
    /// [`WarpgrapherProp`]: struct.WarpgrapherProp.html
    /// [`WarpgrapherType`]: struct.WarpgrapherType.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::{WarpgrapherType, WarpgrapherProp, EndpointsFilter};
    ///
    /// let wt = WarpgrapherType::new(
    ///     "User".to_string(),
    ///     vec!(WarpgrapherProp::new("name".to_string(), "String".to_string(), true, false, None, None),
    ///          WarpgrapherProp::new("role".to_string(), "String".to_string(), true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    /// ```
    pub fn new(
        name: String,
        props: Vec<WarpgrapherProp>,
        rels: Vec<WarpgrapherRel>,
        endpoints: EndpointsFilter,
    ) -> WarpgrapherType {
        WarpgrapherType {
            name,
            props,
            rels,
            endpoints,
        }
    }

    /// Creates a new [`WarpgrapherType`] data structure from
    /// a yaml-formatted string
    pub fn from_yaml(yaml: &str) -> Result<WarpgrapherType, Error> {
        serde_yaml::from_str(yaml)
            .map_err(|e| Error::new(ErrorKind::ConfigDeserializationError(e), None))
    }
}

/// Configuration item for a custom Endpoint
///
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    /// Name of this Endpoint
    pub name: String,

    /// Class of endpoint (Mutation or Query)
    pub class: EndpointClass,

    /// Defines the input of the endpoint
    pub input: Option<EndpointType>,

    /// Defines the type returned by the endpoint
    pub output: EndpointType,
}

impl Endpoint {
    pub fn new(
        name: String,
        class: EndpointClass,
        input: Option<EndpointType>,
        output: EndpointType,
    ) -> Endpoint {
        Endpoint {
            name,
            class,
            input,
            output,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EndpointClass {
    Query,
    Mutation,
}

/// Configuration item for a custom Endpoint
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointType {
    /// Defines option for an endpoint type to use an existing or custom type
    #[serde(rename = "type")]
    pub type_def: WarpgrapherTypeDef,

    /// Determines if the endpoint type is a list
    #[serde(default = "get_false")]
    pub list: bool,

    /// Determine if the type is required (non-nullable)
    #[serde(default = "get_false")]
    pub required: bool,
}

impl EndpointType {
    pub fn new(
        type_def: WarpgrapherTypeDef,
        list: bool,
        required: bool,
    ) -> EndpointType {
        EndpointType {
            type_def,
            list,
            required,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
//#[serde(untagged)]
pub enum GraphqlType {
    Int,
    Float,
    String,
    Boolean,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum WarpgrapherTypeDef {
    Scalar(GraphqlType),
    Existing(String),
    Custom(WarpgrapherType),
}

/// Creates a combined [`Config`] data structure from multiple [`Config`] structs
/// [`Config`]: struct.Config.html
///
/// All [`Config`] must be the same version.
///
/// Will return a Result<Config, Error> with a single [`Config`] struct or a
/// [`ConfigVersionMismatchError`] if the versions across all Config do not match.
///
/// #Example
/// ```rust
///     use warpgrapher::engine::config::{Config, compose};
///
///     let mut config_vec: Vec<Config> = Vec::new();
///     let config = compose(config_vec).unwrap();
/// ```

pub fn compose(configs: Vec<Config>) -> Result<Config, Error> {
    let mut version: Option<i32> = None;
    let mut model: Vec<WarpgrapherType> = Vec::new();
    let mut endpoints: Vec<Endpoint> = Vec::new();

    for c in configs {
        version = match version {
            Some(v) => Some(v),
            None => Some(c.version),
        };

        if let Some(ref v) = version {
            if *v != c.version {
                return Err(Error::new(
                    ErrorKind::ConfigVersionMismatchError(
                        format!("All configs must be the same version. Found {wrong_version}, expected {correct_version}", wrong_version=c.version, correct_version=v),
                        c.version
                    ),
                    None,
                ));
            }
        }

        for m in c.model {
            model.push(m.clone());
        }

        for e in c.endpoints {
            endpoints.push(e.clone());
        }
    }

    let version: i32 = match version {
        Some(v) => v,
        None => 0,
    };

    Ok(Config::new(version, model, endpoints))
}

#[cfg(test)]
mod tests {
    use super::{
        compose, ErrorKind, Config, EndpointsFilter, WarpgrapherProp,
        WarpgrapherType,
    };
    use std::fs::File;
    use std::io::prelude::*;

    /// Passes if a new Configuration is created
    #[test]
    fn new_warpgrapher_config() {
        let c = Config::new(1, Vec::new(), Vec::new());

        assert!(c.version == 1);
        assert!(c.model.is_empty());
    }

    // Passes if a WarpgrapherProp is created and prints correctly
    #[test]
    fn new_property() {
        let p = WarpgrapherProp::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        );

        assert!(p.name == "name");
        assert!(p.type_name == "String");
    }

    /// Passes if a WarpgrapherType is created
    #[test]
    fn new_node_type() {
        let t = WarpgrapherType::new(
            "User".to_string(),
            vec![
                WarpgrapherProp::new(
                    "name".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
                WarpgrapherProp::new(
                    "role".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
            ],
            vec![],
            EndpointsFilter::all(),
        );

        assert!(t.name == "User");
        assert!(t.props.get(0).unwrap().name == "name");
        assert!(t.props.get(1).unwrap().name == "role");
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn test_type_from_yaml() {
        let mut file = File::open("tests/fixtures/types/Project.yml").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let project = WarpgrapherType::from_yaml(&contents).unwrap();
        assert_eq!(project.name, "Project");
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn test_validate() {
        //Test valid config
        let valid_config =
            match Config::from_file("tests/fixtures/test_config_ok.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        assert!(valid_config.validate().is_ok());

        //Test composed config
        let mut config_vec: Vec<Config> = Vec::new();

        let valid_config_0: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_0.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        let valid_config_1: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_1.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        let valid_config_2: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_2.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        config_vec.push(valid_config_0);
        config_vec.push(valid_config_1);
        config_vec.push(valid_config_2);

        let composed_config: Config = match compose(config_vec) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        assert!(composed_config.validate().is_ok());

        //Test duplicate Type
        let duplicate_type_config: Config = match Config::from_file(
            "tests/fixtures/test_config_duplicate_type.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match duplicate_type_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeDuplicateError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test duplicate Endpoint type
        let duplicate_endpoint_config: Config = match Config::from_file(
            "tests/fixtures/test_config_duplicate_endpoint.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match duplicate_endpoint_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointDuplicateError(_, _) => (), //assert!(true),
                _ => panic!(),
            },
        }

        /*
        // TODO: need to move this inside engine::validate() since it is possible for an endpoint
        // input to be an auto-generated type which cannot be introspected from the context of the
        // config alone
        //Test missing Endpoint type
        let missing_endpoint_type_config: Config =
            match Config::from_file("tests/fixtures/test_config_missing_endpoint_type.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        // TODO: ensure correct error is returned
        assert!(missing_endpoint_type_config.validate().is_err());
        */
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_prop_name_id_test() {
        let node_prop_name_id_config: Config = match Config::from_file(
            "tests/fixtures/test_config_node_prop_name_id.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match node_prop_name_id_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::InvalidPropNameID(_) => (), //assert!(true)
                _ => panic!(),
            },
        }

        let rel_prop_name_id_config: Config = match Config::from_file(
            "tests/fixtures/test_config_rel_prop_name_id.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match rel_prop_name_id_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::InvalidPropNameID(_) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_int_test() {
        //Test Scalar Type Name: Int
        let scalar_type_name_int_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_type_name_int.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_int_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Input Name: Int
        let scalar_endpoint_input_type_name_int_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_input_type_name_int.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_input_type_name_int_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointInputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Output Name: Int
        let scalar_endpoint_output_type_name_int_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_output_type_name_int.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_output_type_name_int_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointOutputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_float_test() {
        //Test Scalar Type Name: Float
        let scalar_type_name_float_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_type_name_float.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_float_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Input Name: Float
        let scalar_endpoint_input_type_name_float_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_input_type_name_float.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_input_type_name_float_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointInputTypeScalarNameError(_, _) => (), //assert!(true),
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Output Name: Float
        let scalar_endpoint_output_type_name_float_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_output_type_name_float.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_output_type_name_float_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointOutputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_string_test() {
        //Test Scalar Type Name: String
        let scalar_type_name_string_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_type_name_string.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_string_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Input Name: String
        let scalar_endpoint_input_type_name_string_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_input_type_name_string.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_input_type_name_string_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointInputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Output Name: String
        let scalar_endpoint_output_type_name_string_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_output_type_name_string.yml"
                    .to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_output_type_name_string_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointOutputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_id_test() {
        //Test Scalar Type Name: ID
        let scalar_type_name_id_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_type_name_id.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_id_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Input Name: id
        let scalar_endpoint_input_type_name_id_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_input_type_name_id.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_input_type_name_id_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointInputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Output Name: ID
        let scalar_endpoint_output_type_name_id_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_output_type_name_id.yml".to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_output_type_name_id_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointOutputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_boolean_test() {
        //Test Scalar Type Name: Boolean
        let scalar_type_name_boolean_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_type_name_boolean.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_boolean_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Input Name: Boolean
        let scalar_endpoint_input_type_name_boolean_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_input_type_name_boolean.yml"
                    .to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_input_type_name_boolean_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointInputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }

        //Test Scalar Endpoint Output Name: Boolean
        let scalar_endpoint_output_type_name_boolean_config: Config =
            match Config::from_file(
                "tests/fixtures/test_config_scalar_endpoint_output_type_name_boolean.yml"
                    .to_string(),
            ) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        match scalar_endpoint_output_type_name_boolean_config.validate() {
            Ok(_) => panic!(),
            Err(e) => match e.kind {
                ErrorKind::ConfigEndpointOutputTypeScalarNameError(_, _) => (), //assert!(true)
                _ => panic!(),
            },
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn test_compose() {
        assert!(
            Config::from_file("tests/fixtures/test_config_err.yml".to_string()).is_err()
        );

        assert!(
            Config::from_file("tests/fixtures/test_config_ok.yml".to_string()).is_ok()
        );

        let mut config_vec: Vec<Config> = Vec::new();

        assert!(compose(config_vec.clone()).is_ok());

        let valid_config_0: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_0.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        let valid_config_1: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_1.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        let valid_config_2: Config = match Config::from_file(
            "tests/fixtures/test_config_compose_2.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        let mismatch_version_config: Config = match Config::from_file(
            "tests/fixtures/test_config_with_version_100.yml".to_string(),
        ) {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        config_vec.push(valid_config_0);
        config_vec.push(valid_config_1);
        config_vec.push(valid_config_2);

        assert!(compose(config_vec.clone()).is_ok());

        config_vec.push(mismatch_version_config);
        assert!(compose(config_vec).is_err());
    }
}
