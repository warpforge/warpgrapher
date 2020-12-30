use crate::engine::context::{GraphQLContext, RequestContext};
use crate::engine::database::{NodeQueryVar, QueryFragment, RelQueryVar, Transaction, Comparison};
use crate::engine::objects::resolvers::SuffixGenerator;
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::{Info, PropertyKind, Property};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use inflector::Inflector;
use log::trace;
use std::collections::HashMap;
use std::convert::TryFrom;

pub(super) fn visit_node_create_mutation_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Node<RequestCtx>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_create_mutation_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    let input = if let Some(handlers) = context
        .event_handlers()
        .before_node_create(node_var.label()?)
    {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
    } else {
        input
    };

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        m.keys()
        //.filter(|k| !k.starts_with("_"))
        .try_for_each(|k| {
            let p = itd.property(k)?;
            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator().map_or(Ok(()), |v_name| {
                        validate_input(context.validators(), &v_name, &input)
                    })
                }
                _ => Ok(()), // No validation action to take
            }
        })?
    }

    // iterate over all input fields and if any of them is of type Input,
    // pass it to the inputs group otherwise pass it to the props group
        /*
        let (props, inputs) = m.into_iter().fold(
            (HashMap::<String, Value>::new(), HashMap::<String, Value>::new()),
            |(mut props, mut inputs), (k, v)| {
                if let Ok(p) = itd.property(&k) {
                    if let PropertyKind::Input = p.kind() {
                        inputs.insert(k, v);
                    } else {
                        props.insert(k, v);
                    }
                } else {
                    props.insert(k, v);
                }
                (props, inputs)
            }
        };
        */

    if let Value::Map(m) = input {
        let (props, inputs) = m.into_iter().try_fold(
            (HashMap::<String, Value>::new(), HashMap::<String, Value>::new()),
            |(mut props, mut inputs), (k, v)| {
                match itd.property(&k)?.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        props.insert(k, v);
                    }
                    PropertyKind::Input => {
                        inputs.insert(k, v);
                    }
                    _ => return Err(Error::TypeNotExpected { details: None}),
                }
                Ok((props, inputs))
            },
        )?;

        let node = transaction
            //.create_node::<RequestCtx>(node_var, props, partition_key_opt, info)
            .create_node(node_var, props, partition_key_opt, info)
            .and_then(|n| {
                if let Some(handlers) = context
                    .event_handlers()
                    .after_node_create(node_var.label()?)
                {
                    handlers
                        .iter()
                        .try_fold(vec![n], |v, f| f(v, context, transaction))?
                        .pop()
                        .ok_or_else(|| Error::ResponseItemNotFound {
                            name: "Node from after_node_create handler".to_string(),
                        })
                } else {
                    Ok(n)
                }
            })?;

        if !inputs.is_empty() {
            let mut id_props = HashMap::new();
            id_props.insert("id".to_string(), Comparison::default(node.id()?.clone()) );

            let fragment = transaction.node_read_fragment(Vec::new(), node_var, id_props, sg)?;
            trace!(
                "visit_node_create_mutation_input -- fragment: {:#?}",
                fragment
            );

            inputs.into_iter().try_for_each(|(k, v)| {
                let p = itd.property(&k)?;

                match p.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => Ok(()), // Handled earlier
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            input_array.into_iter().try_for_each(|val| {
                                visit_rel_create_mutation_input::<T, RequestCtx>(
                                    fragment.clone(),
                                    &RelQueryVar::new(
                                        p.name().to_string(),
                                        sg.suffix(),
                                        node_var.clone(),
                                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                    ),
                                    None,
                                    val,
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    sg,
                                    transaction,
                                    context,
                                )?;
                                Ok(())
                            })
                        } else {
                            visit_rel_create_mutation_input::<T, RequestCtx>(
                                fragment.clone(),
                                &RelQueryVar::new(
                                    p.name().to_string(),
                                    sg.suffix(),
                                    node_var.clone(),
                                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                ),
                                None,
                                v,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                sg,
                                transaction,
                                context,
                            )?;
                            Ok(())
                        }
                    }
                    _ => Err(Error::TypeNotExpected { details: None}),
                }
            })?;
        }

        trace!("visit_node_create_muation_input -- returning {:#?}", node);

        Ok(node)
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

pub(super) fn visit_node_delete_input<T, RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error>
where
    T: Transaction<RequestCtx>,
{
    trace!(
        "visit_node_delete_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    let input = if let Some(handlers) = context
        .event_handlers()
        .before_node_delete(node_var.label()?)
    {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let fragment = visit_node_query_input(
            node_var,
            m.remove("MATCH"), // Remove used to take ownership
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        visit_node_delete_mutation_input::<T, RequestCtx>(
            fragment,
            &node_var,
            m.remove("DELETE"),
            &Info::new(
                itd.property("DELETE")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_delete_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error>
where
    RequestCtx: RequestContext,
    T: Transaction<RequestCtx>,
{
    trace!(
        "visit_node_delete_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name()
    );

    let itd = info.type_def()?;

    let nodes =
        //transaction.read_nodes::<RequestCtx>(node_var, query_fragment, partition_key_opt, info)?;
        transaction.read_nodes(node_var, query_fragment, partition_key_opt, info)?;
    if nodes.is_empty() {
        if let Some(handlers) = context
            .event_handlers()
            .after_node_delete(node_var.label()?)
        {
            handlers.iter().try_fold(Vec::new(), |v, f| f(v, context, transaction))?;
        }
        return Ok(0);
    }

    let fragment = transaction.node_read_by_ids_fragment(node_var, &nodes)?;

    if let Some(Value::Map(m)) = input {
        m.into_iter().try_for_each(|(k, v)| {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        input_array.into_iter().try_for_each(|val| {
                            let rel_var = RelQueryVar::new(
                                k.to_string(),
                                sg.suffix(),
                                node_var.clone(),
                                NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                            );
                            visit_rel_delete_input::<T, RequestCtx>(
                                Some(fragment.clone()),
                                &rel_var,
                                val,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                sg,
                                transaction,
                                context,
                            )?;
                            Ok(())
                        })
                    } else {
                        let rel_var = RelQueryVar::new(
                            k.to_string(),
                            sg.suffix(),
                            node_var.clone(),
                            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                        );
                        visit_rel_delete_input::<T, RequestCtx>(
                            Some(fragment.clone()),
                            &rel_var,
                            v,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            sg,
                            transaction,
                            context,
                        )?;

                        Ok(())
                    }
                }
                _ => Err(Error::TypeNotExpected { details: None}),
            }
        })?
    }

    let result = transaction.delete_nodes(fragment, node_var, partition_key_opt);

    if let Some(handlers) = context
        .event_handlers()
        .after_node_delete(node_var.label()?)
    {
        handlers.iter().try_fold(nodes, |v, f| f(v, context, transaction))?;
    }

    result
}

fn visit_node_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<QueryFragment, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_input called -- node_var: {:#?}, input: {:#?}, into.name: {}",
        node_var,
        input,
        info.name()
    );

    if let Value::Map(m) = input {
        let itd = info.type_def()?;

        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string() + "::NEW or ::EXISTING",
            })?;

        let p = itd.property(&k)?;

        match k.as_ref() {
            "NEW" => {
                let node = visit_node_create_mutation_input::<T, RequestCtx>(
                    node_var,
                    v,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    partition_key_opt,
                    sg,
                    transaction,
                    context,
                )?;

                let mut id_props = HashMap::new();
                id_props.insert("id".to_string(), Comparison::default(node.id()?.clone()) );

                Ok(transaction.node_read_fragment(Vec::new(), node_var, id_props, sg)?)
            }
            "EXISTING" => Ok(visit_node_query_input(
                node_var,
                Some(v),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?),
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + &k,
            }),
        }
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

pub(super) fn visit_node_query_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<QueryFragment, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    let itd = info.type_def()?;
    let dst_var = NodeQueryVar::new(None, "dst".to_string(), sg.suffix());

    if let Some(Value::Map(m)) = input {
        let (props, rqfs) = m.into_iter().try_fold(
            (HashMap::new(), Vec::new()),
            |(mut props, mut rqfs), (k, v)| {
                itd.property(&k)
                    .map_err(|e| e)
                    .and_then(|p| match p.kind() {
                        PropertyKind::ScalarComp => {
                            props.insert(k, Comparison::try_from(v)?);
                            Ok((props, rqfs))
                        }
                        PropertyKind::Scalar => {
                            props.insert(k, Comparison::default(v));
                            Ok((props, rqfs))
                        }
                        PropertyKind::Input => {
                            rqfs.push(visit_rel_query_input(
                                None,
                                &RelQueryVar::new(
                                    k.to_string(),
                                    sg.suffix(),
                                    node_var.clone(),
                                    dst_var.clone(),
                                ),
                                Some(v),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                sg,
                                transaction,
                            )?);
                            Ok((props, rqfs))
                        }
                        _ => Err(Error::TypeNotExpected { details: None}),
                    })
            },
        )?;

        transaction.node_read_fragment(rqfs, &node_var, props, sg)
    } else {
        transaction.node_read_fragment(Vec::new(), &node_var, HashMap::new(), sg)
    }
}

pub(super) fn visit_node_update_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    let input = if let Some(handlers) = context
        .event_handlers()
        .before_node_update(node_var.label()?)
    {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let query_fragment = visit_node_query_input(
            node_var,
            m.remove("MATCH"), // Remove used to take ownership
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        visit_node_update_mutation_input::<T, RequestCtx>(
            query_fragment,
            node_var,
            m.remove("SET").ok_or_else(|| {
                // remove() used here to take ownership of the "set" value, not borrow it
                Error::InputItemNotFound {
                    name: "input::SET".to_string(),
                }
            })?,
            &Info::new(
                itd.property("SET")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_update_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name(),
    );

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        m.keys().try_for_each(|k| {
            let p = itd.property(k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator().map_or(Ok(()), |v_name| {
                        validate_input(context.validators(), &v_name, &input)
                    })
                }
                _ => Ok(()), // No validation action to take
            }
        })?;
    }

    if let Value::Map(m) = input {
        let (props, inputs) = m.into_iter().try_fold(
            (HashMap::new(), HashMap::new()),
            |(mut props, mut inputs), (k, v)| {
                match itd.property(&k)?.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        props.insert(k, v);
                    }
                    PropertyKind::Input => {
                        inputs.insert(k, v);
                    }
                    _ => return Err(Error::TypeNotExpected { details: None}),
                }
                Ok((props, inputs))
            },
        )?;

        let nodes = transaction
            //.update_nodes::<RequestCtx>(query_fragment, node_var, props, partition_key_opt, info)
            .update_nodes(query_fragment, node_var, props, partition_key_opt, info)
            .and_then(|n| {
                if let Some(handlers) = context
                    .event_handlers()
                    .after_node_update(node_var.label()?)
                {
                    handlers.iter().try_fold(n, |v, f| f(v, context, transaction))
                } else {
                    Ok(n)
                }
            })?;
        if nodes.is_empty() {
            return Ok(nodes);
        }
        let node_fragment = transaction.node_read_by_ids_fragment(node_var, &nodes)?;

        inputs.into_iter().try_for_each(|(k, v)| {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => Ok(()), // Properties handled above
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        input_array.into_iter().try_for_each(|val| {
                            visit_rel_change_input::<T, RequestCtx>(
                                node_fragment.clone(),
                                &RelQueryVar::new(
                                    k.clone(),
                                    sg.suffix(),
                                    node_var.clone(),
                                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                ),
                                val,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                sg,
                                transaction,
                                context,
                            )
                        })
                    } else {
                        visit_rel_change_input::<T, RequestCtx>(
                            node_fragment.clone(),
                            &RelQueryVar::new(
                                k,
                                sg.suffix(),
                                node_var.clone(),
                                NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                            ),
                            v,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            sg,
                            transaction,
                            context,
                        )
                    }
                }
                _ => Err(Error::TypeNotExpected { details: None}),
            }
        })?;

        Ok(nodes)
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, RequestCtx>(
    src_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<(), Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_change_input called -- src_fragment: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
        src_fragment, rel_var, input, info.name()
    );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input::<T, RequestCtx>(
                src_fragment,
                rel_var,
                None,
                v,
                &Info::new(
                    itd.property("ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;

            Ok(())
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input::<T, RequestCtx>(
                Some(src_fragment),
                rel_var,
                v,
                &Info::new(
                    itd.property("DELETE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;

            Ok(())
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<T, RequestCtx>(
                Some(src_fragment),
                rel_var,
                None,
                v,
                &Info::new(
                    itd.property("UPDATE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;
            Ok(())
        } else {
            Err(Error::InputItemNotFound {
                name: itd.type_name().to_string() + "::ADD|DELETE|UPDATE",
            })
        }
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_create_input<T, RequestCtx>(
    src_var: &NodeQueryVar,
    rel_name: &str,
    props_type_name: Option<&str>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_create_input called -- src_var: {:#?}, rel_name: {}, input: {:#?}, info.name: {}",
        src_var, rel_name, input, info.name()
    );

    let rel_label = src_var.label()?.to_string() + &rel_name.to_string().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_create(&rel_label) {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
        //handlers.iter().try_fold(input, |v, f| f(v))?
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let src_fragment = visit_node_query_input(
            src_var,
            m.remove("MATCH"), // Remove used to take ownership
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        //let nodes = transaction.read_nodes::<RequestCtx>(
        let nodes = transaction.read_nodes(
            src_var,
            src_fragment.clone(),
            partition_key_opt,
            info,
        )?;

        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        let create_input = m.remove("CREATE").ok_or_else(|| {
            // Using remove to take ownership
            Error::InputItemNotFound {
                name: "input::CREATE".to_string(),
            }
        })?;

        match create_input {
            Value::Map(_) => {
                let rel_var = RelQueryVar::new(
                    rel_name.to_string(),
                    sg.suffix(),
                    src_var.clone(),
                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                );
                visit_rel_create_mutation_input::<T, RequestCtx>(
                    src_fragment,
                    &rel_var,
                    props_type_name,
                    create_input,
                    &Info::new(
                        itd.property("CREATE")?.type_name().to_owned(),
                        info.type_defs(),
                    ),
                    partition_key_opt,
                    sg,
                    transaction,
                    context,
                )
            }
            Value::Array(create_input_array) => create_input_array.into_iter().try_fold(
                Vec::new(),
                |mut rels, create_input_value| -> Result<Vec<Rel<RequestCtx>>, Error> {
                    let rel_var = RelQueryVar::new(
                        rel_name.to_string(),
                        sg.suffix(),
                        src_var.clone(),
                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                    );
                    rels.append(&mut visit_rel_create_mutation_input::<T, RequestCtx>(
                        src_fragment.clone(),
                        &rel_var,
                        props_type_name,
                        create_input_value,
                        &Info::new(
                            itd.property("CREATE")?.type_name().to_owned(),
                            info.type_defs(),
                        ),
                        partition_key_opt,
                        sg,
                        transaction,
                        context,
                    )?);

                    Ok(rels)
                },
            ),
            _ => Err(Error::TypeNotExpected { details: None}),
        }
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_create_mutation_input<T, RequestCtx>(
    src_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_create_mutation_input called -- src_fragment: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, info.name: {}",
            src_fragment, rel_var, props_type_name, input, info.name());

    if let Value::Map(mut m) = input {
        let dst_prop = info.type_def()?.property("dst")?;
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::InputItemNotFound {
                name: "dst".to_string(),
            })?;
        let dst_fragment = visit_rel_nodes_mutation_input_union::<T, RequestCtx>(
            rel_var.dst(),
            dst,
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )?;

        let props = match m.remove("props") {
            None => HashMap::new(),
            Some(Value::Map(hm)) => hm,
            Some(_) => return Err(Error::TypeNotExpected { details: None}),
        };

        let rel_label =
            rel_var.src().label()?.to_string() + &rel_var.label().to_title_case() + "Rel";
        transaction
            .create_rels(
                src_fragment,
                dst_fragment,
                rel_var,
                props,
                props_type_name,
                partition_key_opt,
            )
            .and_then(|rels| {
                if let Some(handlers) = context.event_handlers().after_rel_create(&rel_label) {
                    handlers.iter().try_fold(rels, |v, f| f(v, context))
                } else {
                    Ok(rels)
                }
            })
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_delete_input<T, RequestCtx>(
    src_query_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error>
where
    RequestCtx: RequestContext,
    T: Transaction<RequestCtx>,
{
    trace!("visit_rel_delete_input called -- src_query_opt: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
    src_query_opt, rel_var, input, info.name());

    let rel_label = rel_var.src().label()?.to_string() + &rel_var.label().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_delete(&rel_label) {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let fragment = visit_rel_query_input(
            src_query_opt,
            rel_var,
            m.remove("MATCH"), // remove rather than get to take ownership
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let rel_label =
            rel_var.src().label()?.to_string() + &rel_var.label().to_title_case() + "Rel";
        let rels =
            //transaction.read_rels::<RequestCtx>(fragment, rel_var, None, partition_key_opt)?;
            transaction.read_rels(fragment, rel_var, None, partition_key_opt)?;
        if rels.is_empty() {
            if let Some(handlers) = context.event_handlers().after_rel_delete(&rel_label) {
                handlers.iter().try_fold(Vec::new(), |v, f| f(v, context))?;
            }
            return Ok(0);
        }

        let id_fragment = transaction.rel_read_by_ids_fragment(rel_var, &rels)?;

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input::<T, RequestCtx>(
                id_fragment.clone(),
                rel_var.src(),
                src,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            visit_rel_dst_delete_mutation_input::<T, RequestCtx>(
                id_fragment.clone(),
                rel_var.dst(),
                dst,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;
        }

        let result = transaction.delete_rels(id_fragment, rel_var, partition_key_opt);

        if let Some(handlers) = context.event_handlers().after_rel_delete(&rel_label) {
            handlers.iter().try_fold(rels, |v, f| f(v, context))?;
        }

        result
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_delete_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error>
where
    RequestCtx: RequestContext,
    T: Transaction<RequestCtx>,
{
    trace!(
        "visit_rel_dst_delete_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name()
    );

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_delete_mutation_input::<T, RequestCtx>(
            query_fragment,
            node_var,
            Some(v),
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

fn visit_rel_dst_query_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<Option<QueryFragment>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_dst_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            Ok(Some(visit_node_query_input(
                node_var,
                Some(v),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn visit_rel_dst_update_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_dst_update_mutation_input called -- query_fragment: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, input, info.name());

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, RequestCtx>(
            query_fragment,
            &NodeQueryVar::new(Some(k), "dst".to_string(), sg.suffix()),
            v,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

fn visit_rel_nodes_mutation_input_union<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<QueryFragment, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_nodes_mutation_input_union called -- node_var: {:#?}, input: {:#?}, info.name: {},", 
        node_var, input, info.name());

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_input::<T, RequestCtx>(
            &NodeQueryVar::new(
                Some(k.clone()),
                node_var.base().to_string(),
                node_var.suffix().to_string(),
            ),
            v,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

pub(super) fn visit_rel_query_input<T, RequestCtx>(
    src_fragment_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    input_opt: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<QueryFragment, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_query_input called -- src_fragment_opt: {:#?}, rel_var: {:#?}, input_opt: {:#?}, info.name(): {}",
        src_fragment_opt, rel_var, input_opt, info.name());

    let itd = info.type_def()?;
    let src_prop = itd.property("src")?;
    let dst_prop = itd.property("dst")?;

    if let Some(Value::Map(mut m)) = input_opt {
        let mut props = if let Some(Value::Map(rel_props)) = m.remove("props") {
            rel_props
        } else {
            HashMap::new()
        };

        // uses remove in order to take ownership
        if let Some(id) = m.remove("id") {
            props.insert("id".to_owned(), id);
        }

        let mut value_props : HashMap<String, Comparison> = HashMap::new();
        for (k, v) in props.drain() {
            value_props.insert(k.to_string(), Comparison::try_from(v)?);
        }

        // Remove used to take ownership
        let src_fragment_opt = if let Some(src) = m.remove("src") {
            visit_rel_src_query_input(
                rel_var.src(),
                Some(src),
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?
        } else {
            src_fragment_opt
        };

        // Remove used to take ownership
        let dst_query_opt = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                rel_var.dst(),
                Some(dst),
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?
        } else {
            None
        };

        transaction.rel_read_fragment(src_fragment_opt, dst_query_opt, rel_var, value_props, sg)
    } else {
        transaction.rel_read_fragment(None, None, rel_var, HashMap::new(), sg)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_delete_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error>
where
    RequestCtx: RequestContext,
    T: Transaction<RequestCtx>,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name());

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_delete_mutation_input::<T, RequestCtx>(
            query_fragment,
            node_var,
            Some(v),
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_update_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_src_update_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name());

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, RequestCtx>(
            query_fragment,
            node_var,
            v,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            context,
        )
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

fn visit_rel_src_query_input<T, RequestCtx>(
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<Option<QueryFragment>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_src_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name(),
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let fragment = visit_node_query_input(
                node_var,
                Some(v),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?;

            Ok(Some(fragment))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_update_input<T, RequestCtx>(
    src_fragment_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_input called -- src_fragment_opt: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, info.name: {}",
         src_fragment_opt, rel_var, props_type_name, input, info.name());

    let rel_label = rel_var.src().label()?.to_string() + &rel_var.label().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_update(&rel_label) {
        handlers.iter().try_fold(input, |v, f| f(v, context, transaction, info))?
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let fragment = visit_rel_query_input(
            src_fragment_opt,
            &rel_var,
            m.remove("MATCH"), // uses remove to take ownership
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        trace!("visit_rel_update_input -- fragment: {:#?}", fragment);

        if let Some(update) = m.remove("SET") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, RequestCtx>(
                fragment,
                &rel_var,
                props_type_name,
                update,
                &Info::new(
                    itd.property("SET")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )
        } else {
            Err(Error::InputItemNotFound {
                name: "input::SET".to_string(),
            })
        }
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_update_mutation_input<T, RequestCtx>(
    query_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error>
where
    T: Transaction<RequestCtx>,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_mutation_input called -- query_fragment: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, info.name: {}",
         query_fragment, rel_var, props_type_name, input, info.name());

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let props = if let Some(Value::Map(props)) = m.remove("props") {
            props
        } else {
            HashMap::new()
        };

        let rel_label =
            rel_var.src().label()?.to_string() + &rel_var.label().to_title_case() + "Rel";
        let rels = transaction
            //.update_rels::<RequestCtx>(
            .update_rels(
                query_fragment,
                rel_var,
                props,
                props_type_name,
                partition_key_opt,
            )
            .and_then(|rels| {
                if let Some(handlers) = context.event_handlers().after_rel_update(&rel_label) {
                    handlers.iter().try_fold(rels, |v, f| f(v, context))
                } else {
                    Ok(rels)
                }
            })?;
        if rels.is_empty() {
            return Ok(rels);
        }

        let id_fragment = transaction.rel_read_by_ids_fragment(rel_var, &rels)?;

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<T, RequestCtx>(
                id_fragment.clone(),
                rel_var.src(),
                src,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input::<T, RequestCtx>(
                id_fragment,
                dst,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                context,
            )?;
        }

        Ok(rels)
    } else {
        Err(Error::TypeNotExpected { details: None})
    }
}

fn validate_input(validators: &Validators, v: &str, input: &Value) -> Result<(), Error> {
    let func = validators.get(v).ok_or_else(|| Error::ValidatorNotFound {
        name: v.to_string(),
    })?;

    trace!(
        "validate_input Calling input validator function {} for input value {:#?}",
        v,
        input
    );

    func(input)
}
