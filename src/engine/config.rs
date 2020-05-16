//! models and custom GraphQL endpoints.

//use std::fmt;
use super::context::GraphQLContext;
use super::schema::Info;
use crate::engine::value::Value;
use crate::error::{Error, ErrorKind};
use juniper::{Arguments, ExecutionResult, Executor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::slice::Iter;

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

pub type ResolverFunc<GlobalCtx, ReqCtx> =
    fn(&Info, &Arguments, &Executor<GraphQLContext<GlobalCtx, ReqCtx>>) -> ExecutionResult;

pub type Resolvers<GlobalCtx, ReqCtx> = HashMap<String, Box<ResolverFunc<GlobalCtx, ReqCtx>>>;

pub type ValidatorFunc = fn(&Value) -> Result<(), Error>;

pub type Validators = HashMap<String, Box<ValidatorFunc>>;

/// Configuration item for a Warpgrapher data model. The configuration contains
/// the version of the Warpgrapher configuration file format, and a vector of
/// [`Type`] structures.
///
/// [`Type`]: struct.Type.html
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
    version: i32,

    /// A vector of [`Type`] structures, each defining one type in
    /// the data model
    ///
    /// [`Type`]: struct.Type.html
    #[serde(default)]
    model: Vec<Type>,

    /// A vector of [`Endpoint`] structures, each defining an
    /// a custom root endpoint in the graphql schema
    ///
    /// [`Endpoint`]: struct.Endpoint.html
    #[serde(default)]
    endpoints: Vec<Endpoint>,
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
    pub fn new(version: i32, model: Vec<Type>, endpoints: Vec<Endpoint>) -> Config {
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

    pub fn endpoints(&self) -> Iter<Endpoint> {
        self.endpoints.iter()
    }
    pub fn types(&self) -> Iter<Type> {
        self.model.iter()
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
    /// Checks to verify no duplicate [`Endpoint`] or [`Type`], and that the
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
                if let TypeDef::Custom(t) = &input.type_def {
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
            if let TypeDef::Custom(t) = &ep.output.type_def {
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
                TypeDef::Null => { }
                TypeDef::Scalar(_) => { }
                TypeDef::Existing(t) => {
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
                TypeDef::Custom(_) => { }
                TypeDef::Custom(t) => {
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
                TypeDef::Scalar(_) => {}
                TypeDef::Existing(t) => {
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
                TypeDef::Custom(t) => {
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
/// use warpgrapher::engine::config::Prop;
///
/// let p = Prop::new("name".to_string(), "String".to_string(), true, false, None, None);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prop {
    /// Name of the property
    name: String,

    /// The name of the type of the property (e.g. String)
    #[serde(rename = "type")]
    type_name: String,

    /// True if this property is required to be present on this type; false if
    /// the property is optional
    #[serde(default = "get_false")]
    required: bool,

    /// True if this property is a list
    #[serde(default = "get_false")]
    list: bool,

    /// The name of the resolver function to be called when querying for the value of this prop.
    /// If this field is None, the prop resolves the scalar value from the database.
    #[serde(default = "get_none")]
    resolver: Option<String>,

    /// The name of the validator function to be called when creating or modifying the value of
    /// this prop. If this field is None, the prop resolves the scalar value from the database.
    #[serde(default = "get_none")]
    validator: Option<String>,
}

impl Prop {
    /// Creates a new Prop struct. Takes a String for the name of
    /// the property, a String for the type of the property, and a boolean
    /// that, if true, indicates the property is required, and if false, that
    /// the property is optional.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::Prop;
    ///
    /// let p = Prop::new("name".to_string(), "String".to_string(), true, false, None, None);
    /// ```
    pub fn new(
        name: String,
        type_name: String,
        required: bool,
        list: bool,
        resolver: Option<String>,
        validator: Option<String>,
    ) -> Prop {
        Prop {
            name,
            type_name,
            required,
            list,
            resolver,
            validator,
        }
    }

    pub fn list(&self) -> bool {
        self.list
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn resolver(&self) -> &Option<String> {
        &self.resolver
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn validator(&self) -> &Option<String> {
        &self.validator
    }
}

/// Configuration item for a relationship on a GraphQL type
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::config::{Relationship, EndpointsFilter};
///
/// let p = Relationship::new(
///            "teams".to_string(),
///            true,
///            vec!["User".to_string()],
///            vec![],  
///            EndpointsFilter::all()
///         );
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    /// Name of the relationship
    name: String,

    /// True if its a multi-node relationship
    #[serde(default = "get_false")]
    list: bool,

    /// List of possible dst nodes for the relationship. A single element
    /// vector indicates a single-type rel and more than one element
    /// indicates a multi-type relationship.
    nodes: Vec<String>,

    /// Properties of the relationship
    #[serde(default)]
    props: Vec<Prop>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    endpoints: EndpointsFilter,
}

impl Relationship {
    /// Creates a new, empty [`Relationship`] data structure
    pub fn new(
        name: String,
        list: bool,
        nodes: Vec<String>,
        props: Vec<Prop>,
        endpoints: EndpointsFilter,
    ) -> Relationship {
        Relationship {
            name,
            list,
            nodes,
            props,
            endpoints,
        }
    }

    pub fn endpoints(&self) -> &EndpointsFilter {
        &self.endpoints
    }

    pub fn list(&self) -> bool {
        self.list
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nodes(&self) -> Iter<String> {
        self.nodes.iter()
    }

    pub fn nodes_to_vec(&self) -> Vec<String> {
        self.nodes.clone()
    }
    pub fn props_as_slice(&self) -> &[Prop] {
        &self.props
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
    read: bool,

    /// True if a create endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    create: bool,

    /// True if a update endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    update: bool,

    /// True if a delete endpoint should be generated for the corresponding type/rel
    #[serde(default = "get_true")]
    delete: bool,
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

    pub fn create(&self) -> bool {
        self.create
    }

    pub fn delete(&self) -> bool {
        self.delete
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

    pub fn read(&self) -> bool {
        self.read
    }

    pub fn update(&self) -> bool {
        self.update
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
/// use warpgrapher::engine::config::{Type, Prop, EndpointsFilter};
///
/// let wt = Type::new(
///     "User".to_string(),
///     vec!(Prop::new("name".to_string(), "String".to_string(), true, false, None, None),
///          Prop::new("role".to_string(), "String".to_string(), true, false, None, None)),
///     vec!(),
///     EndpointsFilter::all()
/// );
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Type {
    /// Name of this GraphQL type, also used as the Neo4J label for nodes
    name: String,

    /// Vector of properties on this type
    props: Vec<Prop>,

    /// Vector of relationships on this type
    #[serde(default)]
    rels: Vec<Relationship>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    endpoints: EndpointsFilter,
}

impl Type {
    /// Creates a new Type struct. Takes a String name for the type
    /// and a vector of [`Prop`] structs and returns a
    /// [`Type`].
    ///
    /// [`Prop`]: struct.Prop.html
    /// [`Type`]: struct.Type.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::{Type, Prop, EndpointsFilter};
    ///
    /// let wt = Type::new(
    ///     "User".to_string(),
    ///     vec!(Prop::new("name".to_string(), "String".to_string(), true, false, None, None),
    ///          Prop::new("role".to_string(), "String".to_string(), true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    /// ```
    pub fn new(
        name: String,
        props: Vec<Prop>,
        rels: Vec<Relationship>,
        endpoints: EndpointsFilter,
    ) -> Type {
        Type {
            name,
            props,
            rels,
            endpoints,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn endpoints(&self) -> &EndpointsFilter {
        &self.endpoints
    }

    /// Creates a new [`Type`] data structure from
    /// a yaml-formatted string
    pub fn from_yaml(yaml: &str) -> Result<Type, Error> {
        serde_yaml::from_str(yaml)
            .map_err(|e| Error::new(ErrorKind::ConfigDeserializationError(e), None))
    }

    pub fn props(&self) -> Iter<Prop> {
        self.props.iter()
    }

    pub fn props_as_slice(&self) -> &[Prop] {
        &self.props
    }

    pub fn rels(&self) -> Iter<Relationship> {
        self.rels.iter()
    }
}

/// Configuration item for a custom Endpoint
///
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    /// Name of this Endpoint
    name: String,

    /// Class of endpoint (Mutation or Query)
    class: EndpointClass,

    /// Defines the input of the endpoint
    input: Option<EndpointType>,

    /// Defines the type returned by the endpoint
    output: EndpointType,
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

    pub fn class(&self) -> &EndpointClass {
        &self.class
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn input(&self) -> &Option<EndpointType> {
        &self.input
    }

    pub fn output(&self) -> &EndpointType {
        &self.output
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
    type_def: TypeDef,

    /// Determines if the endpoint type is a list
    #[serde(default = "get_false")]
    list: bool,

    /// Determine if the type is required (non-nullable)
    #[serde(default = "get_false")]
    required: bool,
}

impl EndpointType {
    pub fn new(type_def: TypeDef, list: bool, required: bool) -> EndpointType {
        EndpointType {
            type_def,
            list,
            required,
        }
    }

    pub fn list(&self) -> bool {
        self.list
    }
    pub fn required(&self) -> bool {
        self.required
    }

    pub fn type_def(&self) -> &TypeDef {
        &self.type_def
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
pub enum TypeDef {
    Scalar(GraphqlType),
    Existing(String),
    Custom(Type),
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
    let mut model: Vec<Type> = Vec::new();
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
            model.push(m);
        }

        for e in c.endpoints {
            endpoints.push(e);
        }
    }

    let version: i32 = match version {
        Some(v) => v,
        None => 0,
    };

    Ok(Config::new(version, model, endpoints))
}

#[cfg(test)]
pub(crate) fn mock_project_config() -> Config {
    Config::new(1, vec![mock_project_type()], vec![])
}

#[cfg(test)]
pub(crate) fn mock_project_type() -> Type {
    Type::new(
        "Project".to_string(),
        vec![
            Prop::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            ),
            Prop::new(
                "tags".to_string(),
                "String".to_string(),
                false,
                true,
                None,
                None,
            ),
            Prop::new(
                "public".to_string(),
                "Boolean".to_string(),
                true,
                false,
                None,
                None,
            ),
        ],
        vec![
            Relationship::new(
                "owner".to_string(),
                false,
                vec!["User".to_string()],
                vec![Prop::new(
                    "since".to_string(),
                    "String".to_string(),
                    false,
                    false,
                    None,
                    None,
                )],
                EndpointsFilter::all(),
            ),
            Relationship::new(
                "board".to_string(),
                false,
                vec!["ScrumBoard".to_string(), "KanbanBoard".to_string()],
                vec![],
                EndpointsFilter::all(),
            ),
            Relationship::new(
                "commits".to_string(),
                true,
                vec!["Commit".to_string()],
                vec![],
                EndpointsFilter::all(),
            ),
            Relationship::new(
                "issues".to_string(),
                true,
                vec!["Feature".to_string(), "Bug".to_string()],
                vec![],
                EndpointsFilter::all(),
            ),
        ],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_user_type() -> Type {
    Type::new(
        "User".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_kanbanboard_type() -> Type {
    Type::new(
        "KanbanBoard".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_scrumboard_type() -> Type {
    Type::new(
        "ScrumBoard".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_feature_type() -> Type {
    Type::new(
        "Feature".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_bug_type() -> Type {
    Type::new(
        "Bug".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_commit_type() -> Type {
    Type::new(
        "Commit".to_string(),
        vec![Prop::new(
            "name".to_string(),
            "String".to_string(),
            true,
            false,
            None,
            None,
        )],
        vec![],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
pub(crate) fn mock_endpoint_one() -> Endpoint {
    // RegisterUsers(input: [UserCreateMutationInput]): [User]
    Endpoint::new(
        "RegisterUsers".to_string(),
        EndpointClass::Mutation,
        Some(EndpointType::new(
            TypeDef::Existing("UserCreateMutationInput".to_string()),
            true,
            true,
        )),
        EndpointType::new(TypeDef::Existing("User".to_string()), true, true),
    )
}

#[cfg(test)]
pub(crate) fn mock_endpoint_two() -> Endpoint {
    // DisableUser(input: UserQueryInput): User
    Endpoint::new(
        "DisableUser".to_string(),
        EndpointClass::Mutation,
        Some(EndpointType::new(
            TypeDef::Existing("UserQueryInput".to_string()),
            false,
            true,
        )),
        EndpointType::new(TypeDef::Existing("User".to_string()), false, true),
    )
}

#[cfg(test)]
pub(crate) fn mock_endpoint_three() -> Endpoint {
    // ComputeBurndown(input: BurndownFilter): BurndownMetrics
    Endpoint::new(
        "ComputeBurndown".to_string(),
        EndpointClass::Query,
        Some(EndpointType::new(
            TypeDef::Custom(Type::new(
                "BurndownFilter".to_string(),
                vec![Prop::new(
                    "ticket_types".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                )],
                vec![],
                EndpointsFilter::all(),
            )),
            false,
            false,
        )),
        EndpointType::new(
            TypeDef::Custom(Type::new(
                "BurndownMetrics".to_string(),
                vec![Prop::new(
                    "points".to_string(),
                    "Int".to_string(),
                    false,
                    false,
                    None,
                    None,
                )],
                vec![],
                EndpointsFilter::all(),
            )),
            false,
            true,
        ),
    )
}

#[cfg(test)]
pub(crate) fn mock_config() -> Config {
    Config::new(
        1,
        vec![
            mock_project_type(),
            mock_user_type(),
            mock_kanbanboard_type(),
            mock_scrumboard_type(),
            mock_feature_type(),
            mock_bug_type(),
            mock_commit_type(),
        ],
        vec![
            mock_endpoint_one(),
            mock_endpoint_two(),
            mock_endpoint_three(),
        ],
    )
}

#[cfg(test)]
pub(crate) fn mock_endpoints_filter() -> Config {
    Config::new(
        1,
        vec![Type::new(
            "User".to_string(),
            vec![Prop::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            EndpointsFilter::new(false, true, false, false),
        )],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::{compose, Config, EndpointsFilter, ErrorKind, Prop, Type};
    use std::fs::File;
    use std::io::prelude::*;

    /// Passes if a new Configuration is created
    #[test]
    fn new_warpgrapher_config() {
        let c = Config::new(1, Vec::new(), Vec::new());

        assert!(c.version == 1);
        assert!(c.model.is_empty());
    }

    // Passes if a Prop is created and prints correctly
    #[test]
    fn new_property() {
        let p = Prop::new(
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

    /// Passes if a Type is created
    #[test]
    fn new_node_type() {
        let t = Type::new(
            "User".to_string(),
            vec![
                Prop::new(
                    "name".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
                Prop::new(
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
        let project = Type::from_yaml(&contents).unwrap();
        assert_eq!(project.name, "Project");
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn test_validate() {
        //Test valid config
        let valid_config = match Config::from_file("tests/fixtures/test_config_ok.yml".to_string())
        {
            Err(_) => panic!(),
            Ok(wgc) => wgc,
        };

        assert!(valid_config.validate().is_ok());

        //Test composed config
        let mut config_vec: Vec<Config> = Vec::new();

        let valid_config_0: Config =
            match Config::from_file("tests/fixtures/test_config_compose_0.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        let valid_config_1: Config =
            match Config::from_file("tests/fixtures/test_config_compose_1.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        let valid_config_2: Config =
            match Config::from_file("tests/fixtures/test_config_compose_2.yml".to_string()) {
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
        let duplicate_type_config: Config =
            match Config::from_file("tests/fixtures/test_config_duplicate_type.yml".to_string()) {
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
        let node_prop_name_id_config: Config =
            match Config::from_file("tests/fixtures/test_config_node_prop_name_id.yml".to_string())
            {
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
        let scalar_endpoint_input_type_name_int_config: Config = match Config::from_file(
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
        let scalar_endpoint_output_type_name_int_config: Config = match Config::from_file(
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
        let scalar_endpoint_input_type_name_float_config: Config = match Config::from_file(
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
        let scalar_endpoint_output_type_name_float_config: Config = match Config::from_file(
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
        let scalar_endpoint_input_type_name_string_config: Config = match Config::from_file(
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
        let scalar_endpoint_output_type_name_string_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_endpoint_output_type_name_string.yml".to_string(),
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
        let scalar_endpoint_input_type_name_id_config: Config = match Config::from_file(
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
        let scalar_endpoint_output_type_name_id_config: Config = match Config::from_file(
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
        let scalar_endpoint_input_type_name_boolean_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_endpoint_input_type_name_boolean.yml".to_string(),
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
        let scalar_endpoint_output_type_name_boolean_config: Config = match Config::from_file(
            "tests/fixtures/test_config_scalar_endpoint_output_type_name_boolean.yml".to_string(),
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
        assert!(Config::from_file("tests/fixtures/test_config_err.yml".to_string()).is_err());

        assert!(Config::from_file("tests/fixtures/test_config_ok.yml".to_string()).is_ok());

        let mut config_vec: Vec<Config> = Vec::new();

        assert!(compose(config_vec.clone()).is_ok());

        let valid_config_0: Config =
            match Config::from_file("tests/fixtures/test_config_compose_0.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        let valid_config_1: Config =
            match Config::from_file("tests/fixtures/test_config_compose_1.yml".to_string()) {
                Err(_) => panic!(),
                Ok(wgc) => wgc,
            };

        let valid_config_2: Config =
            match Config::from_file("tests/fixtures/test_config_compose_2.yml".to_string()) {
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
