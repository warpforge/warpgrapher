//! Models and custom GraphQL endpoints.

use crate::engine::schema::{rel_name_variants, type_name_variants};
use crate::Error;
use log::trace;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;
use std::slice::Iter;

const LATEST_CONFIG_VERSION: i32 = 2;

// Convenience function for setting serde default value
fn get_false() -> bool {
    false
}

// Convenience function for setting serde default value
fn get_true() -> bool {
    true
}

// Convenience function for setting serde default value
fn get_none() -> Option<String> {
    None
}

/// Configuration for a Warpgrapher data model. The configuration contains the version of the
/// Warpgrapher configuration file format, a vector of [`Type`] structures, and a vector of
/// [`Endpoint`] structures.
///
/// [`Endpoint`]: struct.Endpoint.html
/// [`Type`]: struct.Type.html
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Configuration;
///  
/// let c = Configuration::new(1, Vec::new(), Vec::new());
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// Version of the Warpgrapher configuration file format used
    version: i32,

    /// A vector of [`Type`] structures, each defining one type in the data model
    ///
    /// [`Type`]: struct.Type.html
    #[serde(default)]
    pub model: Vec<Type>,

    /// A vector of [`Endpoint`] structures, each defining a custom root endpoint in the graphql
    /// schema
    ///
    /// [`Endpoint`]: struct.Endpoint.html
    #[serde(default)]
    endpoints: Vec<Endpoint>,
}

impl Configuration {
    /// Creates a new [`Configuration`] data structure with the version, [`Type`] vector, and
    /// [`Endpoint`] vector provided as arguments.
    ///
    /// [`Configuration`]: struct.Configuration.html
    /// [`Endpoint`]: struct.Endpoint.html
    /// [`Type`]: struct.Type.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::Configuration;
    ///
    /// let c = Configuration::new(1, Vec::new(), Vec::new());
    /// ```
    pub fn new(version: i32, model: Vec<Type>, endpoints: Vec<Endpoint>) -> Configuration {
        Configuration {
            version,
            model,
            endpoints,
        }
    }

    /// Returns an iterator over the [`Endpoint`] structs defining custom root endpoints in the
    /// GraphQL schema
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::Configuration;
    ///
    /// let c = Configuration::new(1, Vec::new(), Vec::new());
    /// for e in c.endpoints() {
    ///     let _name = e.name();
    /// }
    /// ```
    pub fn endpoints(&self) -> Iter<Endpoint> {
        self.endpoints.iter()
    }

    /// Returns an iterator over the [`Type`] structs defining types in the GraphQL schema
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::Configuration;
    ///
    /// let c = Configuration::new(1, Vec::new(), Vec::new());
    /// for t in c.types() {
    ///     let _name = t.name();
    /// }
    /// ```
    pub fn types(&self) -> Iter<Type> {
        self.model.iter()
    }

    /// Validates the [`Configuration`] data structure. Checks that there are no duplicate
    /// [`Endpoint`] or [`Type`] items, and that the [`Endpoint`] input/output types are defined
    /// in the model. Returns () if there are no validation errors.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ConfigItemDuplicated`] if there is more than one type or
    /// more than one endpoint that use the same name.
    ///
    /// Returns an [`Error`] variant [`ConfigItemReserved`] if a named configuration item, such as
    /// an endpoint or type, has a name that is a reserved word, such as "ID" or the name of a
    /// GraphQL scalar type.
    ///
    /// [`ConfigItemDuplicated`]: ../../error/enum.Error.html#variant.ConfigItemDuplicated
    /// [`ConfigItemReserved`]: ../../error/enum.Error.html#variant.ConfigItemReserved
    /// [`Error`]: ../../error/enum.Error.html
    ///
    /// # Example
    /// ```rust
    /// #  use warpgrapher::Configuration;
    ///
    /// let config = Configuration::new(1, Vec::new(), Vec::new());
    /// config.validate();
    /// ```
    pub fn validate(&self) -> Result<(), Error> {
        trace!("Config::validate called");

        let scalar_names = ["Int", "Float", "Boolean", "String", "ID"];

        self.model
            .iter()
            .map(|t| {
                if self.model.iter().filter(|t2| t2.name == t.name).count() > 1 {
                    return Err(Error::ConfigItemDuplicated {
                        type_name: t.name.to_string(),
                    });
                }

                let name_variants = type_name_variants(t);
                self.model.iter().try_for_each(|t2| {
                    if name_variants.contains(t2.name()) {
                        Err(Error::ConfigItemDuplicated {
                            type_name: t2.name().to_string(),
                        })
                    } else {
                        Ok(())
                    }
                })?;

                if scalar_names.iter().any(|s| s == &t.name) {
                    return Err(Error::ConfigItemReserved {
                        type_name: t.name.clone(),
                    });
                }

                if t.props.iter().any(|p| p.name().to_uppercase() == "ID") {
                    return Err(Error::ConfigItemReserved {
                        type_name: "ID".to_string(),
                    });
                }

                // Used by Gremlin return format to return the label itself
                if t.props.iter().any(|p| p.name().to_uppercase() == "LABEL") {
                    return Err(Error::ConfigItemReserved {
                        type_name: "label".to_string(),
                    });
                }

                if t.rels
                    .iter()
                    .any(|r| r.props.iter().any(|p| p.name().to_uppercase() == "ID"))
                {
                    return Err(Error::ConfigItemReserved {
                        type_name: "ID".to_string(),
                    });
                }

                // Used by Gremlin return format to return the label itself
                if t.rels
                    .iter()
                    .any(|r| r.props.iter().any(|p| p.name().to_uppercase() == "LABEL"))
                {
                    return Err(Error::ConfigItemReserved {
                        type_name: "label".to_string(),
                    });
                }

                if t.rels
                    .iter()
                    .any(|r| r.props.iter().any(|p| p.name().to_uppercase() == "SRC"))
                {
                    return Err(Error::ConfigItemReserved {
                        type_name: "src".to_string(),
                    });
                }

                if t.rels
                    .iter()
                    .any(|r| r.props.iter().any(|p| p.name().to_uppercase() == "DST"))
                {
                    return Err(Error::ConfigItemReserved {
                        type_name: "dst".to_string(),
                    });
                }

                t.rels.iter().try_for_each(|r| {
                    let rel_name_variants = rel_name_variants(t, r);

                    self.model.iter().try_for_each(|t2| {
                        if rel_name_variants.contains(t2.name()) {
                            Err(Error::ConfigItemDuplicated {
                                type_name: t2.name().to_string(),
                            })
                        } else {
                            Ok(())
                        }
                    })
                })?;

                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        self.endpoints
            .iter()
            .map(|ep| {
                // Check for duplicate endpoints
                if self.endpoints.iter().filter(|e| e.name == ep.name).count() > 1 {
                    return Err(Error::ConfigItemDuplicated {
                        type_name: ep.name.to_string(),
                    });
                }

                // Check for endpoint custom input using reserved names (GraphQL scalars)
                if let Some(input) = &ep.input {
                    if let TypeDef::Custom(t) = &input.type_def {
                        if scalar_names.iter().any(|s| s == &t.name) {
                            return Err(Error::ConfigItemReserved {
                                type_name: t.name.to_string(),
                            });
                        }
                    }
                }

                // Check for endpoint custom input using reserved names (GraphQL scalars)
                if let TypeDef::Custom(t) = &ep.output.type_def {
                    if scalar_names.iter().any(|s| s == &t.name) {
                        return Err(Error::ConfigItemReserved {
                            type_name: t.name.to_string(),
                        });
                    }
                }

                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(())
    }

    /// Returns the version number of the configuration format used for the configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::Configuration;
    ///
    /// let c = Configuration::new(1, Vec::new(), Vec::new());
    ///
    /// assert_eq!(1, c.version());
    /// ```
    pub fn version(&self) -> i32 {
        self.version
    }
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration {
            version: 1,
            model: vec![],
            endpoints: vec![],
        }
    }
}

impl TryFrom<File> for Configuration {
    type Error = Error;

    fn try_from(f: File) -> Result<Configuration, Error> {
        let r = BufReader::new(f);
        Ok(serde_yaml::from_reader(r)?)
    }
}

impl TryFrom<String> for Configuration {
    type Error = Error;

    fn try_from(s: String) -> Result<Configuration, Error> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl TryFrom<&str> for Configuration {
    type Error = Error;

    fn try_from(s: &str) -> Result<Configuration, Error> {
        Ok(serde_yaml::from_str(s)?)
    }
}

/// Configuration item for custom endpoints
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
/// #   GraphqlType};
///
/// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
/// ```
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
    /// Creates a new configuration item for a custom GraphQL endpoint
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
    /// #   GraphqlType};
    ///
    /// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
    ///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
    /// ```
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

    /// Returns the [`EndpointClass`] of a custom GraphQL endpoint, indicating whether the custom
    /// endpoint is a Query or Mutation.
    ///
    /// [`EndpointClass`]: ./enum.EndpointClass.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
    /// #   GraphqlType};
    ///
    /// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
    ///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
    ///
    /// assert_eq!(&EndpointClass::Query, e.class());
    /// ```
    pub fn class(&self) -> &EndpointClass {
        &self.class
    }

    /// Returns the name of a custom GraphQL endpoint.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
    /// #   GraphqlType};
    ///
    /// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
    ///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
    ///
    /// assert_eq!("CountItems", e.name());
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional type definition of the input to a custom endpoint. A value of None
    /// indicates that the GraphQL endpoint does not take an input.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
    /// #   GraphqlType};
    ///
    /// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
    ///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
    ///
    /// assert!(e.input().is_none());
    /// ```
    pub fn input(&self) -> Option<&EndpointType> {
        self.input.as_ref()
    }

    /// Returns the type definition of the output for a custom endpoint
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Endpoint, EndpointClass, EndpointType, TypeDef,
    /// #   GraphqlType};
    ///
    /// let e = Endpoint::new("CountItems".to_string(), EndpointClass::Query, None,
    ///     EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true));
    ///
    /// assert_eq!(&EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true),
    ///     e.output());
    /// ```
    pub fn output(&self) -> &EndpointType {
        &self.output
    }
}

impl TryFrom<&str> for Endpoint {
    type Error = Error;

    /// Creates a new Endpoint struct from a yaml-formatted string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`YamlDeserializationFailed`] if the yaml-formatted
    /// string is improperly formatted.
    ///
    /// [`YamlDeserializationFailed`]: ../../error/enum.Error.html#variant.YamlDeserializationFailed
    /// [`Error`]: ../../error/enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::{Endpoint};
    ///
    /// use std::convert::TryFrom;
    /// let t = Endpoint::try_from("
    /// name: RegisterUser
    /// class: Mutation
    /// input:
    ///   type: UserInput
    /// output:
    ///   type: User
    /// ").unwrap();
    /// ```
    fn try_from(yaml: &str) -> Result<Endpoint, Error> {
        serde_yaml::from_str(yaml).map_err(|e| Error::YamlDeserializationFailed { source: e })
    }
}

/// Determines whether a custom GraphQL endpoint is a query or mutation endpoint
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::EndpointClass;
///
/// let ec = EndpointClass::Query;
/// ```
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EndpointClass {
    /// Indicates that a custome GraphQL endpoint should be a query
    Query,

    /// Indicates that a cutom GraphQL endpoint should be a mutation
    Mutation,
}

/// Configuration item for endpoint filters. This allows configuration to control which of the
/// basic create, read, update, and delete (CRUD) operations are auto-generated for a [`Type`] or a
/// [`Relationship`]. If a filter boolean is set to true, the operation is generated. False
/// indicates that the operation should not be generated.
///
/// [`Relationship`]: ./struct.Relationship.html
/// [`Type`]: ./struct.Type.html
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::EndpointsFilter;
///
/// let ef = EndpointsFilter::new(true, true, true, true);
/// ```
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::new(true, true, false, false);
    /// ```
    pub fn new(read: bool, create: bool, update: bool, delete: bool) -> EndpointsFilter {
        EndpointsFilter {
            read,
            create,
            update,
            delete,
        }
    }

    /// Creates a new filter with all endpoints -- create, read, update, and delete
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::all();
    /// ```
    pub fn all() -> EndpointsFilter {
        EndpointsFilter {
            read: true,
            create: true,
            update: true,
            delete: true,
        }
    }

    /// Returns true if Warpgrapher should generate a create operation for the relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::all();
    /// assert_eq!(true, ef.create());
    /// ```
    pub fn create(self) -> bool {
        self.create
    }

    /// Returns true if Warpgrapher should generate a delete operation for the relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::all();
    /// assert_eq!(true, ef.delete());
    /// ```
    pub fn delete(self) -> bool {
        self.delete
    }

    /// Creates a new filter with all endpoints -- create, read, update, and delete -- filtered out
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::none();
    /// ```
    pub fn none() -> EndpointsFilter {
        EndpointsFilter {
            read: false,
            create: false,
            update: false,
            delete: false,
        }
    }

    /// Returns true if Warpgrapher should generate a read operation for the relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::all();
    /// assert_eq!(true, ef.read());
    /// ```
    pub fn read(self) -> bool {
        self.read
    }

    /// Returns true if Warpgrapher should generate a update operation for the relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::EndpointsFilter;
    ///
    /// let ef = EndpointsFilter::all();
    /// assert_eq!(true, ef.update());
    /// ```
    pub fn update(self) -> bool {
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

/// Configuration item describing a type used with a custom GraphQL endpoint, either as the input
/// to the custom endpoint, or as its output
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{EndpointType, GraphqlType, TypeDef};
///
/// let et = EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true);
/// ```
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
    /// Creates a new [`EndpointType`] configuration item, describing either the input or output
    /// type of a custom GraphQL endpoint
    ///
    /// [`EndpointType`]: ./struct.EndpointType.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointType, GraphqlType, TypeDef};
    ///
    /// let et = EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true);
    /// ```
    pub fn new(type_def: TypeDef, list: bool, required: bool) -> EndpointType {
        EndpointType {
            type_def,
            list,
            required,
        }
    }

    /// True if the type associated with a custom GraphQL endpoint is a list of elements; false if
    /// the type is a single value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointType, GraphqlType, TypeDef};
    ///
    /// let et = EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true);
    ///
    /// assert_eq!(false, et.list());
    /// ```
    pub fn list(&self) -> bool {
        self.list
    }

    /// True if the type associated with a custom GraphQL endpoint is required; false if it is
    /// optional
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointType, GraphqlType, TypeDef};
    ///
    /// let et = EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true);
    ///
    /// assert_eq!(true, et.required());
    /// ```
    pub fn required(&self) -> bool {
        self.required
    }

    /// Returns a [`TypeDef`] enumeration, describing whether the type is a GraphQL scalar type,
    /// an existing type defined elsewhere in the configuration, or a new custom type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointType, GraphqlType, TypeDef};
    ///
    /// let et = EndpointType::new(TypeDef::Scalar(GraphqlType::Int), false, true);
    ///
    /// assert_eq!(&TypeDef::Scalar(GraphqlType::Int), et.type_def());
    /// ```
    pub fn type_def(&self) -> &TypeDef {
        &self.type_def
    }
}

/// Enumeration representing Graphql scalar types
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::GraphqlType;
///
/// let gt = GraphqlType::Int;
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum GraphqlType {
    /// GraphQL integer
    Int,

    /// GraphQL floating point number
    Float,

    /// GraphQL string value
    String,

    /// GraphQL boolean value
    Boolean,
}

/// Configuration item for a property on a GraphQL type, modeled as properties on a graph node.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{Property, UsesFilter};
///
/// let p = Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true,
/// false, None, None);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    /// Name of the property
    name: String,

    /// Filter of properties that determines whether the property will be used in creation,
    /// matching/query, update, and output/shape portions of the schema
    #[serde(default)]
    uses: UsesFilter,

    /// The name of the type of the property (e.g. String)
    #[serde(rename = "type")]
    type_name: String,

    /// True if this property is required to be present on this type; false if the property is
    /// optional
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

impl Property {
    /// Creates a new Property struct.
    ///
    /// # Arguments
    ///
    /// * a String for the name of the property
    /// * a String for the type of the property
    /// * a boolean that, if true, indicatees that the property is mandatory, and if false, that
    /// the property is optional
    /// * a boolean that, if true, indicates that the property is a list of scalers, and if false,
    /// that the property is a single value
    /// * an optional string providing the name of a resolver, if the property is a dynamic
    /// property with a custom resolver, and
    /// * an optional string providing the name of a custom validator
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Property, UsesFilter};
    ///
    /// let p = Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true,
    /// false, None, None);
    /// ```
    pub fn new(
        name: String,
        uses: UsesFilter,
        type_name: String,
        required: bool,
        list: bool,
        resolver: Option<String>,
        validator: Option<String>,
    ) -> Property {
        Property {
            name,
            uses,
            type_name,
            required,
            list,
            resolver,
            validator,
        }
    }

    /// Returns a boolean that if true, indicates that this property contains a list of scalar
    /// values, and if false, indicates that the property contains only one value (or potentially
    /// zero values if required is also false).
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("name".to_string(), UsesFiter::all(), "String".to_string(),
    ///         true, false, None, None);
    ///
    /// assert!(!p.list());
    pub fn list(&self) -> bool {
        self.list
    }

    /// Returns the name of the property
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("propname".to_string(), UsesFilter::all(), "String".to_string(),
    ///     true, false, None, None);
    ///
    /// assert_eq!("propname", p.name());
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the filter describing how a property is to be used
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("propname".to_string(), UsesFilter::all(), "String".to_string(),
    ///         true, false, None, None);
    ///
    /// assert_eq!(UsesFilter::all(), p.uses());
    pub fn uses(&self) -> UsesFilter {
        self.uses
    }

    /// Returns the optional name of the custom resolver associated with this property
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("propname".to_string(), "String".to_string(), true, false,
    ///     Some("CustomResolver".to_string()), None);
    ///
    /// assert_eq!("CustomResolver", p.resolver().unwrap());
    pub fn resolver(&self) -> Option<&String> {
        self.resolver.as_ref()
    }

    /// Returns a boolean that if true, indicates that this property is mandatory, and if false,
    /// that the property is not required, and may be absent.
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("name".to_string(), "String".to_string(), true, false, None, None);
    ///
    /// assert!(p.required());
    pub fn required(&self) -> bool {
        self.required
    }

    /// Returns the name of the type of the property
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("propname".to_string(), "String".to_string(), true, false, None,
    ///     None);
    ///
    /// assert_eq!("String", p.type_name());
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the optional name of the custom validator associated with this property
    ///
    /// # Examples
    ///
    /// # use warpgrapher::engine::config::Property;
    ///
    /// let p = Property::new("propname".to_string(), "String".to_string(), true, false,
    ///     None, Some("CustomValidator".to_string()));
    ///
    /// assert_eq!("CustomValidator", p.validator().unwrap());
    pub fn validator(&self) -> Option<&String> {
        self.validator.as_ref()
    }
}

/// Configuration item for a relationship on a GraphQL type
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{Relationship, EndpointsFilter};
///
/// let r = Relationship::new(
///     "teams".to_string(),
///     true,
///     vec!["User".to_string()],
///     vec![],  
///     EndpointsFilter::all(),
///     None
/// );
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
    props: Vec<Property>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    endpoints: EndpointsFilter,

    /// The name of the resolver function to be called when querying for the value of this prop.
    /// If this field is None, the prop resolves the scalar value from the database.
    #[serde(default = "get_none")]
    resolver: Option<String>,
}

impl Relationship {
    /// Creates a new Relationship struct.
    ///
    /// # Arguments
    ///
    /// * a String for the name of the relationship
    /// * a boolean that, if true, indicates that the property is a list, and if false, that the
    /// relationship is to a single node
    /// * a list of possible destination node types for the relationship. A single element in the
    /// list indicates that this is a single-type relationship, whereas more than one element
    /// indicates a multi-type relationship.
    /// * an [`EndpointsFilter`] struct indicating which of the standard operations (create, read,
    /// update, and delete) should be generated for this relationship
    /// * an optional string providing the name of a resolver, if the relationship is a dynamic
    /// relationship with a custom resolver
    ///
    /// [`EndpointsFilter`]: ./struct.EndpointsFilter.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("name".to_string(), false, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    /// ```
    pub fn new(
        name: String,
        list: bool,
        nodes: Vec<String>,
        props: Vec<Property>,
        endpoints: EndpointsFilter,
        resolver: Option<String>,
    ) -> Relationship {
        Relationship {
            name,
            list,
            nodes,
            props,
            endpoints,
            resolver,
        }
    }

    /// Returns the [`EndpointsFilter`] struct that indicates which of the four basic Create, Read,
    /// Update, and Delete (CRUD) operations Warpgrapher should auto-generate for this
    /// relationship.
    ///
    /// [`EndpointsFilter`]: ./struct.EndpointsFilter.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("name".to_string(), false, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert_eq!(&EndpointsFilter::all(), r.endpoints());
    /// ```
    pub fn endpoints(&self) -> &EndpointsFilter {
        &self.endpoints
    }

    /// Returns true if the relationship is a list, indicating a one-to-many (or many-to-many)
    /// relationship. Returns false if the node can only have one relationship of this type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("name".to_string(), true, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert!(r.list());
    /// ```
    pub fn list(&self) -> bool {
        self.list
    }

    /// Returns the name of the relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("RelName".to_string(), true, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert_eq!("RelName", r.name());
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an iterator over the names of the Warpgrapher [`Type`] definitions that are
    /// possible destination nodes for this relationship.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("RelName".to_string(), true, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert_eq!(1, r.nodes().count());
    /// assert_eq!("User", r.nodes().next().expect("Expected an element"));
    /// ```
    pub fn nodes(&self) -> Iter<String> {
        self.nodes.iter()
    }

    /// Returns a slice of [`Property`] references for properties associated with the relationship.
    ///
    /// [`Property`]: ./struct.Property.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("RelName".to_string(), true, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert_eq!(0, r.props_as_slice().len())
    /// ```
    pub fn props_as_slice(&self) -> &[Property] {
        &self.props
    }

    /// Returns an option for a string containing the name of a custom resolver, if this
    /// relationship is resolved by a custom resolver instead of an auto-generated read resolver.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Relationship};
    ///
    /// let r = Relationship::new("RelName".to_string(), true, vec!["User".to_string()], vec![],
    ///     EndpointsFilter::all(), None);
    ///
    /// assert!(r.resolver().is_none())
    /// ```
    pub fn resolver(&self) -> Option<&String> {
        self.resolver.as_ref()
    }
}

/// Configuration item for a GraphQL type. In back-end storage, the type is recorded in a label
/// attached to the graph node.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
///
/// let t = Type::new(
///     "User".to_string(),
///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(),
///         true, false, None, None),
///          Property::new("role".to_string(), UsesFilter::all(), "String".to_string(),
///         true, false, None, None)),
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
    #[serde(default)]
    props: Vec<Property>,

    /// Vector of relationships on this type
    #[serde(default)]
    rels: Vec<Relationship>,

    /// Filter of endpoints that determines which CRUD endpoints will be
    /// auto generated for the relationship
    #[serde(default)]
    endpoints: EndpointsFilter,
}

impl Type {
    /// Creates a new Type struct.
    ///
    /// # Arguments
    ///
    /// * name - the name of the type, which will be recorded as the label on a node in the graph
    /// back-end
    /// * props - a vector of [`Property`] structs describing the properties on the node
    /// * rels - a vector of [`Relationship`] structs describing the outgoing relationships from
    /// this node type
    /// * endpoints - an [`EndpointsFilter`] struct describing which CRUD operations should be
    /// generated automatically for this node type.
    ///
    /// [`EndpointsFilter`]: ./struct.EndpointsFilter.html
    /// [`Property`]: ./struct.Property.html
    /// [`Type`]: ./struct.Type.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(),
    ///         true, false, None, None),
    ///          Property::new("role".to_string(), UsesFilter::all(), "String".to_string(),
    ///         true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    /// ```
    pub fn new(
        name: String,
        props: Vec<Property>,
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

    /// Returns the name of the type. This type name is used as the label on nodes of this type in
    /// the graph database storage back-end.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None),
    ///          Property::new("role".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    ///
    /// assert_eq!("User", t.name());
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the [`EndpointsFilter`] struct associate with this type, determining which CRUD
    /// operations should be auto-generated for this node type.
    ///
    /// [`EndpointsFilter`]: ./struct.EndpointsFilter.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None),
    ///          Property::new("role".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    ///
    /// assert_eq!(&EndpointsFilter::all(), t.endpoints());
    /// ```
    pub fn endpoints(&self) -> &EndpointsFilter {
        &self.endpoints
    }

    /// Returns an iterator over the [`Property`] structs defining properties on this node type.
    ///
    /// [`Property`]: ./struct.Property.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(),
    ///         true, false, None, None)), vec!(), EndpointsFilter::all());
    ///
    /// assert_eq!("name", t.props().next().expect("Expected property").name());
    /// ```
    pub fn props(&self) -> Iter<Property> {
        self.props.iter()
    }

    pub fn mut_props(&mut self) -> &mut Vec<Property> {
        &mut self.props
    }

    /// Returns a slice of the [`Property`] structs defining properties on this node type.
    ///
    /// [`Property`]: ./struct.Property.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{Type, Property, EndpointsFilter, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None)),
    ///     vec!(),
    ///     EndpointsFilter::all()
    /// );
    ///
    /// assert_eq!("name", t.props_as_slice()[0].name());
    /// ```
    pub fn props_as_slice(&self) -> &[Property] {
        &self.props
    }

    /// Returns an iterator over the [`Relationship`] structs defining relationships originating
    /// from this node type.
    ///
    /// [`Relationship`]: ./struct.Relationship.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::{EndpointsFilter, Property, Relationship,
    /// #    Type, UsesFilter};
    ///
    /// let t = Type::new(
    ///     "User".to_string(),
    ///     vec!(Property::new("name".to_string(), UsesFilter::all(), "String".to_string(), true,
    ///         false, None, None)),
    ///     vec!(Relationship::new("rel_name".to_string(), false, vec!("Role".to_string()), vec!(
    ///         Property::new("rel_prop".to_string(), UsesFilter::all(), "String".to_string(), true, false, None, None)
    ///     ), EndpointsFilter::all(), None)),
    ///     EndpointsFilter::all()
    /// );
    ///
    /// assert_eq!("rel_name", t.rels().next().expect("Expected relationship").name());
    /// ```
    pub fn rels(&self) -> Iter<Relationship> {
        self.rels.iter()
    }
}

impl TryFrom<&str> for Type {
    type Error = Error;

    /// Creates a new Type struct from a yaml-formatted string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`YamlDeserializationFailed`] if the yaml-formatted
    /// string is improperly formatted.
    ///
    /// [`YamlDeserializationFailed`]: ../../error/enum.Error.html#variant.YamlDeserializationFailed
    /// [`Error`]: ../../error/enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::config::{Type};
    ///
    /// use std::convert::TryFrom;
    /// let t = Type::try_from("
    /// name: User
    /// props:
    ///   - name: name
    ///     type: String
    /// ").unwrap();
    /// ```
    fn try_from(yaml: &str) -> Result<Type, Error> {
        serde_yaml::from_str(yaml).map_err(|e| Error::YamlDeserializationFailed { source: e })
    }
}

/// Enumeration representing the definition of a type used as the optional input or the output for
/// a custom GraphQL endpooint
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{GraphqlType, TypeDef};
///
/// let td = TypeDef::Scalar(GraphqlType::Int);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum TypeDef {
    /// The type is a GraphQL scalar type. The tuple value indicates the scalar type to use.
    Scalar(GraphqlType),

    /// The type is an existing type, already defined by the configuration file or auto-generated
    /// by Warpgrapher. The tuple value is the name of the existing type to use.
    Existing(String),

    /// The type is a new custom type, defined for this custom endpoint. The tuple value is a
    /// [`Type`] struct defining the custom type.alloc
    ///
    /// [`Type`]: ./struct.Type.html
    Custom(Type),
}

/// Configuration item for property usage filters. This allows configuration to control which of the
/// basic creation input, query input, update input, and output uses are auto-generated for a
/// [`Property`]. If a filter boolean is set to true, the use of the property is generated. False
/// indicates that the property should be omitted from use. For example, one might set the output
/// use to true and all other uses to false for calculated value that is derived upon request but
/// would never appear in the creation or update of a node. If all are set to false, the property
/// is hidden, meaning that it can be read from and written to the database but does not appear in
/// the client-facing GraphQL schema.
///
/// [`Property`]: ./struct.Property.html
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::UsesFilter;
///
/// let uf = UsesFilter::new(true, true, true, true);
/// ```
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UsesFilter {
    /// True if the property should be included in the NodeCreateMutationInput portion of the schema
    #[serde(default = "get_true")]
    create: bool,

    /// True if the property should be included in the NodeQueryInput portion of the schema
    #[serde(default = "get_true")]
    query: bool,

    /// True if the property should be included in the NodeUpdateMutationInput portion of the schema
    #[serde(default = "get_true")]
    update: bool,

    /// True if the property should be included in output shape portion of the schema
    #[serde(default = "get_true")]
    output: bool,
}

impl UsesFilter {
    /// Creates a new filter with the option to configure uses of a property
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::new(false, false, false, true);
    /// ```
    pub fn new(create: bool, query: bool, update: bool, output: bool) -> UsesFilter {
        UsesFilter {
            create,
            query,
            update,
            output,
        }
    }

    /// Creates a new filter with all uses -- create, query, update, and output
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::all();
    /// ```
    pub fn all() -> UsesFilter {
        UsesFilter {
            create: true,
            query: true,
            update: true,
            output: true,
        }
    }

    /// Returns true if Warpgrapher should use the property in the NodeCreateMutationInput
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::all();
    /// assert_eq!(true, uf.create());
    /// ```
    pub fn create(self) -> bool {
        self.create
    }

    /// Returns true if Warpgrapher should use the property in the NodeQueryInput
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::all();
    /// assert_eq!(true, uf.query());
    /// ```
    pub fn query(self) -> bool {
        self.query
    }

    /// Creates a new filter with no uses of a property, hiding it from the GraphQL schema
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::none();
    /// ```
    pub fn none() -> UsesFilter {
        UsesFilter {
            create: false,
            query: false,
            update: false,
            output: false,
        }
    }

    /// Returns true if Warpgrapher should generate a property in the output shape of a node
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::all();
    /// assert_eq!(true, uf.output());
    /// ```
    pub fn output(self) -> bool {
        self.output
    }

    /// Returns true if Warpgrapher should use the property in the NodeUpdateInput
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::config::UsesFilter;
    ///
    /// let uf = UsesFilter::all();
    /// assert_eq!(true, uf.update());
    /// ```
    pub fn update(self) -> bool {
        self.update
    }
}

impl Default for UsesFilter {
    fn default() -> UsesFilter {
        UsesFilter {
            create: true,
            query: true,
            update: true,
            output: true,
        }
    }
}

/// Creates a combined [`Configuration`] data structure from multiple [`Configuration`] structs.
/// All [`Configuration`] structs must be the same version.
///
/// Returns a `Result<Configuration, Error>` with a single [`Configuration`] struct, or if the
/// versions across all `Configuration`s do not match, a [`ConfigVersionMismatched`] error.
///
/// [`Configuration`]: struct.Configuration.html
/// [`ConfigVersionMismatched`]: ../enum.Error.html#variant.ConfigVersionMismatched
///
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::engine::config::{Configuration, compose};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut config_vec: Vec<Configuration> = Vec::new();
/// let config = compose(config_vec)?;
/// # Ok(())
/// # }
/// ```
pub fn compose(configs: Vec<Configuration>) -> Result<Configuration, Error> {
    let mut version: Option<i32> = None;
    let mut model: Vec<Type> = Vec::new();
    let mut endpoints: Vec<Endpoint> = Vec::new();

    configs
        .into_iter()
        .map(|mut c| {
            match version {
                None => version = Some(c.version()),
                Some(v) => {
                    if v != c.version {
                        return Err(Error::ConfigVersionMismatched {
                            expected: v,
                            found: c.version,
                        });
                    }
                }
            }

            model.append(&mut c.model);
            endpoints.append(&mut c.endpoints);
            Ok(())
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // There will be no version number if the vector of Configurations is empty, in which case
    // we might as well use the latest version
    Ok(Configuration::new(
        version.unwrap_or(LATEST_CONFIG_VERSION),
        model,
        endpoints,
    ))
}

#[cfg(test)]
pub(crate) fn mock_project_config() -> Configuration {
    Configuration::new(1, vec![mock_project_type()], vec![])
}

#[cfg(test)]
pub(crate) fn mock_project_type() -> Type {
    Type::new(
        "Project".to_string(),
        vec![
            Property::new(
                "name".to_string(),
                UsesFilter::all(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            ),
            Property::new(
                "tags".to_string(),
                UsesFilter::all(),
                "String".to_string(),
                false,
                true,
                None,
                None,
            ),
            Property::new(
                "public".to_string(),
                UsesFilter::all(),
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
                vec![Property::new(
                    "since".to_string(),
                    UsesFilter::all(),
                    "String".to_string(),
                    false,
                    false,
                    None,
                    None,
                )],
                EndpointsFilter::all(),
                None,
            ),
            Relationship::new(
                "board".to_string(),
                false,
                vec!["ScrumBoard".to_string(), "KanbanBoard".to_string()],
                vec![],
                EndpointsFilter::all(),
                None,
            ),
            Relationship::new(
                "commits".to_string(),
                true,
                vec!["Commit".to_string()],
                vec![],
                EndpointsFilter::all(),
                None,
            ),
            Relationship::new(
                "issues".to_string(),
                true,
                vec!["Feature".to_string(), "Bug".to_string()],
                vec![],
                EndpointsFilter::all(),
                None,
            ),
        ],
        EndpointsFilter::all(),
    )
}

#[cfg(test)]
fn mock_user_type() -> Type {
    Type::new(
        "User".to_string(),
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
        vec![Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
                vec![Property::new(
                    "ticket_types".to_string(),
                    UsesFilter::all(),
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
                vec![Property::new(
                    "points".to_string(),
                    UsesFilter::all(),
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
pub(crate) fn mock_config() -> Configuration {
    Configuration::new(
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
pub(crate) fn mock_endpoints_filter() -> Configuration {
    Configuration::new(
        1,
        vec![Type::new(
            "User".to_string(),
            vec![Property::new(
                "name".to_string(),
                UsesFilter::all(),
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
    use super::{
        compose, Configuration, Endpoint, EndpointType, EndpointsFilter, Property, Relationship,
        Type, UsesFilter,
    };
    use crate::Error;
    use std::convert::TryInto;
    use std::fs::File;

    /// There's not really much of a "test" per se, in this first unit test.
    /// This is the example used in the book/src/warpgrapher/config.md file, so
    /// having it here is a forcing function to make sure we catch any changes
    /// in interface that would change this code and update the book to match.
    #[test]
    fn warpgrapher_book_config() {
        let config = Configuration::new(
            1,
            vec![
                // User
                Type::new(
                    "User".to_string(),
                    vec![
                        Property::new(
                            "username".to_string(),
                            UsesFilter::all(),
                            "String".to_string(),
                            false,
                            false,
                            None,
                            None,
                        ),
                        Property::new(
                            "email".to_string(),
                            UsesFilter::all(),
                            "String".to_string(),
                            false,
                            false,
                            None,
                            None,
                        ),
                    ],
                    Vec::new(),
                    EndpointsFilter::all(),
                ),
                // Team
                Type::new(
                    "Team".to_string(),
                    vec![Property::new(
                        "teamname".to_string(),
                        UsesFilter::all(),
                        "String".to_string(),
                        false,
                        false,
                        None,
                        None,
                    )],
                    vec![Relationship::new(
                        "members".to_string(),
                        true,
                        vec!["User".to_string()],
                        Vec::new(),
                        EndpointsFilter::default(),
                        None,
                    )],
                    EndpointsFilter::all(),
                ),
            ],
            vec![],
        );

        assert!(!config.model.is_empty());
    }

    /// Passes if a new Configuration is created
    #[test]
    fn new_warpgrapher_config() {
        let c = Configuration::new(1, Vec::new(), Vec::new());

        assert!(c.version == 1);
        assert!(c.model.is_empty());
    }

    // Passes if a Property is created and prints correctly
    #[test]
    fn new_property() {
        let p = Property::new(
            "name".to_string(),
            UsesFilter::all(),
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
                Property::new(
                    "name".to_string(),
                    UsesFilter::all(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
                Property::new(
                    "role".to_string(),
                    UsesFilter::all(),
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
    fn test_validate() {
        //Test valid config
        let valid_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_ok.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        assert!(valid_config.validate().is_ok());

        //Test composed config
        let mut config_vec: Vec<Configuration> = Vec::new();

        let valid_config_0: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_0.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        let valid_config_1: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_1.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        let valid_config_2: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_2.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        config_vec.push(valid_config_0);
        config_vec.push(valid_config_1);
        config_vec.push(valid_config_2);

        let composed_config: Configuration = match compose(config_vec) {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        assert!(composed_config.validate().is_ok());

        //Test duplicate Type
        let duplicate_type_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_duplicate_type.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match duplicate_type_config.validate() {
            Err(Error::ConfigItemDuplicated { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test duplicate Endpoint type
        let duplicate_endpoint_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_duplicate_endpoint.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match duplicate_endpoint_config.validate() {
            Err(Error::ConfigItemDuplicated { type_name: _ }) => (),
            _ => panic!(),
        }

        let duplicate_derived_name_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_duplicate_derived_name.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match duplicate_derived_name_config.validate() {
            Err(Error::ConfigItemDuplicated { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_prop_name_id_test() {
        let node_prop_name_id_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_node_prop_name_id.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match node_prop_name_id_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        let rel_prop_name_id_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_rel_prop_name_id.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match rel_prop_name_id_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_prop_name_src_test() {
        let rel_prop_name_src_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_rel_prop_name_src.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match rel_prop_name_src_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_prop_name_dst_test() {
        let rel_prop_name_dst_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_rel_prop_name_dst.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        match rel_prop_name_dst_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_int_test() {
        //Test Scalar Type Name: Int
        let scalar_type_name_int_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_type_name_int.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_int_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Input Name: Int
        let scalar_endpoint_input_type_name_int_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_input_type_name_int.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_input_type_name_int_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Output Name: Int
        let scalar_endpoint_output_type_name_int_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_output_type_name_int.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_output_type_name_int_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_float_test() {
        //Test Scalar Type Name: Float
        let scalar_type_name_float_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_type_name_float.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_float_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Input Name: Float
        let scalar_endpoint_input_type_name_float_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_input_type_name_float.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_input_type_name_float_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Output Name: Float
        let scalar_endpoint_output_type_name_float_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_output_type_name_float.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_output_type_name_float_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_string_test() {
        //Test Scalar Type Name: String
        let scalar_type_name_string_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_type_name_string.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_string_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Input Name: String
        let scalar_endpoint_input_type_name_string_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_input_type_name_string.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_input_type_name_string_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Output Name: String
        let scalar_endpoint_output_type_name_string_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_output_type_name_string.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_output_type_name_string_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_id_test() {
        //Test Scalar Type Name: ID
        let scalar_type_name_id_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_type_name_id.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_id_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Input Name: id
        let scalar_endpoint_input_type_name_id_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_input_type_name_id.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_input_type_name_id_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Output Name: ID
        let scalar_endpoint_output_type_name_id_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_output_type_name_id.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_output_type_name_id_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn config_scalar_name_boolean_test() {
        //Test Scalar Type Name: Boolean
        let scalar_type_name_boolean_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_type_name_boolean.yml",
        )
        .expect("Coudln't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_type_name_boolean_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Input Name: Boolean
        let scalar_endpoint_input_type_name_boolean_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_input_type_name_boolean.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_input_type_name_boolean_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }

        //Test Scalar Endpoint Output Name: Boolean
        let scalar_endpoint_output_type_name_boolean_config: Configuration = match File::open(
            "tests/fixtures/config-validation/test_config_scalar_endpoint_output_type_name_boolean.yml",
        )
        .expect("Couldn't open file")
        .try_into()
        {
            Err(e) => panic!("{}", e),
            Ok(wgc) => wgc,
        };

        match scalar_endpoint_output_type_name_boolean_config.validate() {
            Err(Error::ConfigItemReserved { type_name: _ }) => (),
            _ => panic!(),
        }
    }

    #[allow(clippy::match_wild_err_arm)]
    #[test]
    fn test_compose() {
        assert!(TryInto::<Configuration>::try_into(
            File::open("tests/fixtures/config-validation/test_config_err.yml")
                .expect("Couldn't open file")
        )
        .is_err());

        assert!(TryInto::<Configuration>::try_into(
            File::open("tests/fixtures/config-validation/test_config_ok.yml")
                .expect("Couldn't open file")
        )
        .is_ok());

        let mut config_vec: Vec<Configuration> = Vec::new();

        assert!(compose(config_vec.clone()).is_ok());

        let valid_config_0: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_0.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        let valid_config_1: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_1.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        let valid_config_2: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_compose_2.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        let mismatch_version_config: Configuration =
            match File::open("tests/fixtures/config-validation/test_config_with_version_100.yml")
                .expect("Couldn't open file")
                .try_into()
            {
                Err(e) => panic!("{}", e),
                Ok(wgc) => wgc,
            };

        config_vec.push(valid_config_0);
        config_vec.push(valid_config_1);
        config_vec.push(valid_config_2);

        assert!(compose(config_vec.clone()).is_ok());

        config_vec.push(mismatch_version_config);
        assert!(compose(config_vec).is_err());
    }

    /// Passes if Configuration implements the Send trait
    #[test]
    fn test_config_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Configuration>();
    }

    /// Passes if Configuration implements the Sync trait
    #[test]
    fn test_config_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Configuration>();
    }

    /// Passes if Endpoint implements the Send trait
    #[test]
    fn test_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Endpoint>();
    }

    /// Passes if Endpoint implements the Sync trait
    #[test]
    fn test_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Endpoint>();
    }

    /// Passes if EndpointsFilter implements the Send trait
    #[test]
    fn test_endpoints_filter_send() {
        fn assert_send<T: Send>() {}
        assert_send::<EndpointsFilter>();
    }

    /// Passes if EndpointsFilter implements the Sync trait
    #[test]
    fn test_endpoints_filter_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<EndpointsFilter>();
    }

    /// Passes if EndpointType implements the Send trait
    #[test]
    fn test_endpoints_type_send() {
        fn assert_send<T: Send>() {}
        assert_send::<EndpointType>();
    }

    /// Passes if EndpointType implements the Sync trait
    #[test]
    fn test_endpoints_type_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<EndpointType>();
    }

    /// Passes if Property implements the Send trait
    #[test]
    fn test_property_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Property>();
    }

    /// Passes if Property implements the Sync trait
    #[test]
    fn test_property_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Property>();
    }

    /// Passes if Relationship implements the Send trait
    #[test]
    fn test_relationship_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Relationship>();
    }

    /// Passes if Relationship implements the Sync trait
    #[test]
    fn test_relationship_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Relationship>();
    }

    /// Passes if Type implements the Send trait
    #[test]
    fn test_type_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Type>();
    }

    /// Passes if Type implements the Sync trait
    #[test]
    fn test_type_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Type>();
    }
}
