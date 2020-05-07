use super::{
    get_env_string, get_env_u16, DatabaseEndpoint, DatabasePool, QueryResult, Transaction,
};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
use crate::server::value::Value;
use crate::{Error, ErrorKind};
#[cfg(feature = "graphson2")]
// use gremlin_client::process::traversal::traversal;
#[cfg(feature = "graphson2")]
use gremlin_client::{ConnectionOptions, GKey, GValue, GraphSON, GremlinClient, ToGValue, GID};
use juniper::FieldError;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

pub struct Graphson2Endpoint {
    host: String,
    port: u16,
    login: String,
    pass: String,
}

impl Graphson2Endpoint {
    pub fn from_env() -> Result<Graphson2Endpoint, Error> {
        Ok(Graphson2Endpoint {
            host: get_env_string("WG_GRAPHSON2_HOST")?,
            port: get_env_u16("WG_GRAPHSON2_PORT")?,
            login: get_env_string("WG_GRAPHSON2_LOGIN")?,
            pass: get_env_string("WG_GRAPHSON2_PASS")?,
        })
    }
}

impl DatabaseEndpoint for Graphson2Endpoint {
    fn get_pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Graphson2(
            GremlinClient::connect(
                ConnectionOptions::builder()
                    .host(&self.host)
                    .port(self.port)
                    .pool_size(num_cpus::get().try_into().unwrap_or(8))
                    .ssl(true)
                    .serializer(GraphSON::V1)
                    .deserializer(GraphSON::V1)
                    .credentials(&self.login, &self.pass)
                    .build(),
            )
            .map_err(|e| Error::new(ErrorKind::CouldNotBuildGraphson2Pool(e), None))?,
        ))
    }
}

pub struct Graphson2Transaction {
    client: GremlinClient,
}

impl Graphson2Transaction {
    pub fn new(client: GremlinClient) -> Graphson2Transaction {
        Graphson2Transaction { client }
    }
}

impl Transaction for Graphson2Transaction {
    type ImplQueryResult = Graphson2QueryResult;

    fn begin(&self) -> Result<(), FieldError> {
        Ok(())
    }

    fn commit(&mut self) -> Result<(), FieldError> {
        Ok(())
    }

    fn create_node(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        let mut query = String::from("g.addV('") + label + "')";
        query.push_str(".property('partitionKey', partitionKey)");
        for (k, _v) in props.iter() {
            query.push_str(".property('");
            query.push_str(k);
            query.push_str("', ");
            query.push_str(k);
            query.push_str(")");
        }

        self.exec(&query, partition_key_opt, Some(props))
    }

    fn create_rels(
        &mut self,
        src_label: &str,
        src_ids: Value,
        dst_label: &str,
        dst_ids: Value,
        rel_name: &str,
        params: &mut HashMap<String, Value>,
        partition_key_opt: &Option<String>,
    ) -> Result<Self::ImplQueryResult, FieldError> {
        let mut props = HashMap::new();
        if let Some(Value::Map(pm)) = params.remove("props") {
            // remove rather than get to take ownership
            for (k, v) in pm.into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        if let (Value::Array(src_id_vec), Value::Array(dst_id_vec)) = (src_ids, dst_ids) {
            let mut query = String::from("g.V()");
            query.push_str(".has('partitionKey', partitionKey)");
            query.push_str(&(String::from(".hasLabel('") + dst_label + "')"));
            query.push_str(&String::from(".has('id', within("));

            for (i, id) in dst_id_vec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        query.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        query.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidType("Expected IDs to be strings".to_string()),
                        None,
                    )
                    .into());
                }
            }

            query.push_str(&(String::from(")).as('dst')")));
            query.push_str(&(String::from(".V()")));
            query.push_str(&(String::from(".has('partitionKey', partitionKey)")));
            query.push_str(&(String::from(".hasLabel('") + src_label + "')"));
            query.push_str(&(String::from(".has('id', within(")));

            for (i, id) in src_id_vec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        query.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        query.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidType("Expected IDs to be strings".to_string()),
                        None,
                    )
                    .into());
                }
            }

            query.push_str(&String::from("))"));
            query.push_str(&(String::from(".addE('") + rel_name + "').to('dst')"));
            query.push_str(".property('partitionKey', partitionKey)");
            for (k, _v) in props.iter() {
                query.push_str(".property('");
                query.push_str(k);
                query.push_str("', ");
                query.push_str(k);
                query.push_str(")");
            }

            self.exec(&query, partition_key_opt, Some(props))
        } else {
            Err(Error::new(
                ErrorKind::InvalidType("Expected ID array".to_string()),
                None,
            )
            .into())
        }
    }

    fn delete_nodes(
        &mut self,
        _label: &str,
        ids: Value,
        partition_key_opt: &Option<String>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        if let Value::Array(idvec) = ids {
            let mut qs = String::from("g.V().has('id', within(");
            let length = idvec.len();

            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidType("Expected IDs to be strings".to_string()),
                        None,
                    )
                    .into());
                }
            }

            qs.push_str(")).drop()");

            self.exec(&qs, partition_key_opt, None)?;

            let mut v: Vec<GValue> = Vec::new();
            v.push(GValue::Int32(length as i32));
            Ok(Graphson2QueryResult::new(v))
        } else {
            Err(Error::new(
                ErrorKind::InvalidType("Expected ID array".to_string()),
                None,
            )
            .into())
        }
    }

    fn exec(
        &mut self,
        query: &str,
        partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        trace!(
            "Graphson2Transaction::exec called -- query: {}, partition_key: {:#?}, param: {:#?}",
            query,
            partition_key_opt,
            params
        );

        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            let pms = params.unwrap_or_else(HashMap::new);
            for (k, v) in pms.iter() {
                param_list.push((k.as_str(), v))
            }
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice());
            let results = raw_results?;

            let mut v = Vec::new();
            for r in results {
                v.push(r?);
            }

            Ok(Graphson2QueryResult::new(v))
        } else {
            Err(Error::new(ErrorKind::MissingPartitionKey, None).into())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn node_query_string(
        &mut self,
        // query_string: &str,
        rel_query_fragments: Vec<String>,
        params: &mut HashMap<String, Value>,
        label: &str,
        _var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        trace!(
            "transaction::node_query_string called, union_type: {:#?}",
            union_type
        );

        let mut qs = String::new();

        if return_node {
            qs.push_str("g.V()");
        }

        if !union_type {
            qs.push_str(&(String::from(".hasLabel('") + label + "')"));
        }

        qs.push_str(".has('partitionKey', partitionKey)");

        for (k, v) in props.into_iter() {
            qs.push_str(&(String::from(".has('") + &k + "', " + &k + param_suffix + ")"));
            params.insert(k + param_suffix, v);
        }

        if !rel_query_fragments.is_empty() {
            qs.push_str(".where(");

            if rel_query_fragments.len() > 1 {
                qs.push_str("and(");
            }

            for (i, rqf) in rel_query_fragments.iter().enumerate() {
                if i == 0 {
                    qs.push_str(&(String::from("outE()") + rqf));
                } else {
                    qs.push_str(&(String::from(", outE()") + rqf));
                }
            }

            if rel_query_fragments.len() > 1 {
                qs.push_str(")");
            }

            qs.push_str(")");
        }

        trace!("node_query_string -- query_string: {}", qs);
        Ok(qs)
    }

    fn rel_query_string(
        &mut self,
        // query: &str,
        src_label: &str,
        src_suffix: &str,
        src_ids_opt: Option<Value>,
        src_query_opt: Option<String>,
        rel_name: &str,
        _dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
        params: &mut HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        let mut qs = String::new();

        if return_rel {
            qs.push_str("g.E()");
        }

        qs.push_str(&(String::from(".hasLabel('") + rel_name + "')"));
        qs.push_str(".has('partitionKey', partitionKey)");

        for (k, v) in props.into_iter() {
            qs.push_str(&(String::from(".has('") + &k + ", " + &k + src_suffix + dst_suffix + ")"));
            params.insert(k + src_suffix + dst_suffix, v);
        }
        qs.push_str(".where(");

        if dst_query_opt.is_some() {
            qs.push_str("and(");
        }

        if let Some(src_query) = src_query_opt {
            qs.push_str(&(String::from("outV()") + &src_query));
        } else {
            qs.push_str(&(String::from("outV().hasLabel('") + src_label + "')"));
        }

        if let Some(Value::String(id)) = src_ids_opt {
            qs.push_str(&(String::from(".has('id', '") + &id + "')"));
        } else if let Some(Value::Array(idvec)) = src_ids_opt {
            qs.push_str(".has('id', within(");

            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidType("Expected IDs to be strings".to_string()),
                        None,
                    )
                    .into());
                }
            }

            qs.push_str("))");
        }

        if let Some(dst_query) = dst_query_opt {
            qs.push_str(", ");
            qs.push_str(&(String::from("inV()") + &dst_query));
            qs.push_str(")");
        }

        qs.push_str(")");

        if return_rel {
            qs.push_str(".project('r', 'src', 'dst').by(__).by(outV()).by(inV())")
        }

        trace!("rel_query_string -- query_string: {}", qs);
        Ok(qs)
    }

    fn rollback(&mut self) -> Result<(), FieldError> {
        Ok(())
    }

    fn update_nodes(
        &mut self,
        label: &str,
        ids: Value,
        props: HashMap<String, Value>,
        partition_key_opt: &Option<String>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        trace!("transaction::update_nodes called, label: {}, ids: {:#?}, props: {:#?}, partition_key_opt: {:#?}", label, ids, props, partition_key_opt);

        if let Value::Array(idvec) = ids {
            let mut qs = String::from("g.V().has('id', within(");

            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidType("Expected IDs to be strings".to_string()),
                        None,
                    )
                    .into());
                }
            }

            qs.push_str("))");

            for k in props.keys() {
                qs.push_str(&(String::from(".property('") + k + "', " + k + ")"));
            }

            self.exec(&qs, partition_key_opt, Some(props))
        } else {
            Err(Error::new(
                ErrorKind::InvalidType("Expected ID array".to_string()),
                None,
            )
            .into())
        }
    }
}

#[derive(Debug)]
pub struct Graphson2QueryResult {
    results: Vec<GValue>,
}

impl Graphson2QueryResult {
    pub fn new(results: Vec<GValue>) -> Graphson2QueryResult {
        Graphson2QueryResult { results }
    }
}

impl QueryResult for Graphson2QueryResult {
    fn get_nodes<GlobalCtx, ReqCtx>(
        self,
        _name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        trace!(
            "Graphson2QueryResult::get_nodes self.results: {:#?}",
            self.results
        );

        let mut v = Vec::new();
        for result in self.results {
            if let GValue::Vertex(vertex) = result {
                let mut fields = HashMap::new();
                fields.insert(
                    "id".to_string(),
                    Value::String(match vertex.id() {
                        GID::Int32(i) => i.to_string(),
                        GID::Int64(i) => i.to_string(),
                        GID::String(s) => s.to_string(),
                    }),
                );

                let label = vertex.label().to_string();

                for (key, vertex_property_list) in vertex.into_iter() {
                    fields.insert(
                        key.to_owned(),
                        vertex_property_list
                            .into_iter()
                            .next()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::MissingResultElement("Vertex Property".to_string()),
                                    None,
                                )
                            })?
                            .try_into()?,
                    );
                }

                v.push(Node::new(label, fields))
            } else {
                return Err(
                    Error::new(ErrorKind::InvalidType(format!("{:#?}", result)), None).into(),
                );
            }
        }

        trace!("Graphson2QueryResult::get_nodes returning {:#?}", v);

        Ok(v)
    }

    fn get_rels<GlobalCtx, ReqCtx>(
        self,
        _src_name: &str,
        _src_suffix: &str,
        _rel_name: &str,
        _dst_name: &str,
        _dst_suffix: &str,
        props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        /*
        trace!(
            "Graphson2QueryResult::get_rels self.results: {:#?}",
            self.results
        );
        */

        let mut v = Vec::new();
        for result in self.results {
            if let GValue::Map(m) = result {
                let mut hm = HashMap::new();
                for (k, v) in m.into_iter() {
                    hm.insert(k, v);
                }

                if let (
                    Some(GValue::Edge(edge)),
                    Some(GValue::Vertex(out_vertex)),
                    Some(GValue::Vertex(in_vertex)),
                ) = (
                    hm.remove(&GKey::String("r".to_string())),
                    hm.remove(&GKey::String("src".to_string())),
                    hm.remove(&GKey::String("dst".to_string())),
                ) {
                    let mut src_fields = HashMap::new();
                    src_fields.insert(
                        "id".to_string(),
                        Value::String(match out_vertex.id() {
                            GID::Int32(i) => i.to_string(),
                            GID::Int64(i) => i.to_string(),
                            GID::String(s) => s.to_string(),
                        }),
                    );

                    let src_label = out_vertex.label().to_string();

                    for (key, vertex_property_list) in out_vertex.into_iter() {
                        src_fields.insert(
                            key.to_owned(),
                            vertex_property_list
                                .into_iter()
                                .next()
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::MissingResultElement(
                                            "Vertex Property".to_string(),
                                        ),
                                        None,
                                    )
                                })?
                                .try_into()?,
                        );
                    }

                    let mut dst_fields = HashMap::new();
                    dst_fields.insert(
                        "id".to_string(),
                        Value::String(match in_vertex.id() {
                            GID::Int32(i) => i.to_string(),
                            GID::Int64(i) => i.to_string(),
                            GID::String(s) => s.to_string(),
                        }),
                    );

                    let dst_label = in_vertex.label().to_string();

                    for (key, vertex_property_list) in in_vertex.into_iter() {
                        dst_fields.insert(
                            key.to_owned(),
                            vertex_property_list
                                .into_iter()
                                .next()
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::MissingResultElement(
                                            "Vertex Property".to_string(),
                                        ),
                                        None,
                                    )
                                })?
                                .try_into()?,
                        );
                    }

                    let rel_id = Value::String(match edge.id() {
                        GID::Int32(i) => i.to_string(),
                        GID::Int64(i) => i.to_string(),
                        GID::String(s) => s.to_string(),
                    });

                    let _rel_label = edge.label().to_string();

                    let mut rel_fields = HashMap::new();
                    for (key, property) in edge.into_iter() {
                        rel_fields.insert(key.to_owned(), property.take::<GValue>()?.try_into()?);
                    }

                    v.push(Rel::new(
                        rel_id,
                        match props_type_name {
                            Some(p_type_name) => {
                                Some(Node::new(p_type_name.to_string(), rel_fields))
                            }
                            None => None,
                        },
                        Node::new(src_label, src_fields),
                        Node::new(dst_label, dst_fields),
                    ));
                } else {
                    return Err(Error::new(
                        ErrorKind::MissingResultElement("Rel, src, or dst".to_string()),
                        None,
                    )
                    .into());
                }
            } else {
                return Err(
                    Error::new(ErrorKind::InvalidType(format!("{:#?}", result)), None).into(),
                );
            }
        }

        trace!("Graphson2QueryResult::get_rels returning {:#?}", v);

        Ok(v)
    }

    fn get_ids(&self, _type_name: &str) -> Result<Value, FieldError> {
        trace!(
            "Graphson2QueryResult::get_ids self.results: {:#?}",
            self.results
        );
        let mut v = Vec::new();
        for result in &self.results {
            if let GValue::Vertex(vertex) = result {
                v.push(Value::String(match vertex.id() {
                    GID::String(s) => s.to_string(),
                    GID::Int32(i) => i.to_string(),
                    GID::Int64(i) => i.to_string(),
                }));
            } else {
                return Err(
                    Error::new(ErrorKind::InvalidType(format!("{:#?}", result)), None).into(),
                );
            }
        }
        Ok(Value::Array(v))
    }

    fn get_count(&self) -> Result<i32, FieldError> {
        trace!(
            "Graphson2QueryResult::get_count self.results: {:#?}",
            self.results
        );

        if let Some(GValue::Int32(i)) = self.results.get(0) {
            Ok(*i)
        } else {
            Err(Error::new(
                ErrorKind::InvalidType("Expected int32 for count".to_string()),
                None,
            )
            .into())
        }
    }

    fn len(&self) -> i32 {
        trace!(
            "Graphson2QueryResult::len self.results: {:#?}",
            self.results
        );
        0
    }

    fn is_empty(&self) -> bool {
        trace!(
            "Graphson2QueryResult::is_empty self.results: {:#?}",
            self.results
        );
        true
    }
}
