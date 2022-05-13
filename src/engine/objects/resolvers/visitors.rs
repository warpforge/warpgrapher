use crate::engine::context::{GraphQLContext, RequestContext};
use crate::engine::database::{
    Comparison, CrudOperation, NodeQueryVar, QueryFragment, RelQueryVar, Transaction,
};
use crate::engine::database::{DatabaseEndpoint, DatabasePool};
use crate::engine::events::EventFacade;
use crate::engine::objects::resolvers::SuffixGenerator;
use crate::engine::objects::{Node, Options, Rel};
use crate::engine::schema::{Info, PropertyKind};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use inflector::Inflector;
use juniper::BoxFuture;
use log::trace;
use std::collections::HashMap;
use std::convert::TryFrom;

pub(crate) fn visit_node_create_mutation_input<'a, RequestCtx: RequestContext>(
    node_var: &'a NodeQueryVar,
    mut input: Value,
    options: Options,
    info: &'a Info,
    sg: &'a mut SuffixGenerator,
    transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &'a GraphQLContext<RequestCtx>,
) -> BoxFuture<'a, Result<Node<RequestCtx>, Error>> {
    Box::pin(async move {
        trace!(
        "visit_node_create_mutation_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
        );

        if let Some(handlers) = context
            .event_handlers()
            .before_node_create(node_var.label()?)
        {
            for f in handlers.iter() {
                input = f(
                    input,
                    EventFacade::new(
                        CrudOperation::CreateNode(node_var.label()?.to_string()),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }

        let itd = info.type_def()?;

        if let Value::Map(ref m) = input {
            m.keys().try_for_each(|k| {
                let p = itd.property(k)?;
                match p.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        p.validator().map_or(Ok(()), |v_name| {
                            validate_input(context.validators(), v_name, &input)
                        })
                    }
                    _ => Ok(()), // No validation action to take
                }
            })?
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
                        _ => {
                            return Err(Error::TypeNotExpected {
                                details: Some("Expected Scalar or Input".to_string()),
                            })
                        }
                    }
                    Ok((props, inputs))
                },
            )?;

            let node = transaction
                .create_node(node_var, props, options.clone(), info, sg)
                .await?;

            let node = if let Some(handlers) = context
                .event_handlers()
                .after_node_create(node_var.label()?)
            {
                let mut v = vec![node];
                for f in handlers.iter() {
                    v = f(
                        v,
                        EventFacade::new(
                            CrudOperation::CreateNode(node_var.label()?.to_string()),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
                v.pop().ok_or_else(|| Error::ResponseItemNotFound {
                    name: "Node from after_node_create handler".to_string(),
                })?
            } else {
                node
            };

            if !inputs.is_empty() {
                let mut id_props = HashMap::new();
                id_props.insert("id".to_string(), Comparison::default(node.id()?.clone()));

                let fragment =
                    transaction.node_read_fragment(Vec::new(), node_var, id_props, sg)?;
                trace!(
                    "visit_node_create_mutation_input -- fragment: {:#?}",
                    fragment
                );

                for (k, v) in inputs.into_iter() {
                    let p = itd.property(&k)?;

                    match p.kind() {
                        PropertyKind::Scalar | PropertyKind::DynamicScalar => (), // Handled earlier
                        PropertyKind::Input => {
                            if let Value::Array(input_array) = v {
                                for val in input_array.into_iter() {
                                    visit_rel_create_mutation_input::<RequestCtx>(
                                        fragment.clone(),
                                        &RelQueryVar::new(
                                            p.name().to_string(),
                                            sg.suffix(),
                                            node_var.clone(),
                                            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                        ),
                                        val,
                                        options.clone(),
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        sg,
                                        transaction,
                                        context,
                                    )
                                    .await?;
                                }
                            } else {
                                visit_rel_create_mutation_input::<RequestCtx>(
                                    fragment.clone(),
                                    &RelQueryVar::new(
                                        p.name().to_string(),
                                        sg.suffix(),
                                        node_var.clone(),
                                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                    ),
                                    v,
                                    options.clone(),
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    sg,
                                    transaction,
                                    context,
                                )
                                .await?;
                            }
                        }
                        _ => {
                            return Err(Error::TypeNotExpected {
                                details: Some(
                                    "Expected Scalar, DynamicScalar, or Input".to_string(),
                                ),
                            })
                        }
                    }
                }
            }

            let node = if let Some(handlers) = context
                .event_handlers()
                .after_subgraph_create(node_var.label()?)
            {
                let mut v = vec![node];
                for f in handlers.iter() {
                    v = f(
                        v,
                        EventFacade::new(
                            CrudOperation::CreateNode(node_var.label()?.to_string()),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
                v.pop().ok_or_else(|| Error::ResponseItemNotFound {
                    name: "Node from after_subgraph_create handler".to_string(),
                })?
            } else {
                node
            };

            trace!("visit_node_create_muation_input -- returning {:#?}", node);

            Ok(node)
        } else {
            Err(Error::TypeNotExpected {
                details: Some(
                    "Expected visit_node_create_mutation_input input to be Map".to_string(),
                ),
            })
        }
    })
}

pub(crate) async fn visit_node_delete_input<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    mut input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error> {
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
        for f in handlers.iter() {
            input = f(
                input,
                EventFacade::new(
                    CrudOperation::DeleteNode(node_var.label()?.to_string()),
                    context,
                    transaction,
                    info,
                ),
            )
            .await?
        }
        input
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let fragment = visit_node_query_input::<RequestCtx>(
            node_var,
            m.remove("MATCH"), // Remove used to take ownership
            options.clone(),
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
        )
        .await?;

        visit_node_delete_mutation_input::<RequestCtx>(
            fragment,
            node_var,
            m.remove("DELETE"),
            options,
            &Info::new(
                itd.property("DELETE")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected {
            details: Some("Expected visit_node_delete_input input to be Map".to_string()),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_delete_mutation_input<'a, RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    node_var: &'a NodeQueryVar,
    input: Option<Value>,
    options: Options,
    info: &'a Info,
    sg: &'a mut SuffixGenerator,
    transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &'a GraphQLContext<RequestCtx>,
) -> BoxFuture<'a, Result<i32, Error>> {
    Box::pin(async move {
        trace!(
        "visit_node_delete_mutation_input called -- query_fragment: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}",
        query_fragment, node_var, input, info.name()
    );

        let itd = info.type_def()?;

        let mut nodes = transaction
            .read_nodes(node_var, query_fragment, options.clone(), info)
            .await?;
        if nodes.is_empty() {
            if let Some(handlers) = context
                .event_handlers()
                .after_node_delete(node_var.label()?)
            {
                let mut v = Vec::new();
                for f in handlers.iter() {
                    v = f(
                        v,
                        EventFacade::new(
                            CrudOperation::DeleteNode(node_var.label()?.to_string()),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
            }
            return Ok(0);
        }

        let fragment = transaction.node_read_by_ids_fragment(node_var, &nodes)?;

        if let Some(Value::Map(m)) = input {
            for (k, v) in m.into_iter() {
                let p = itd.property(&k)?;

                match p.kind() {
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            for val in input_array.into_iter() {
                                let rel_var = RelQueryVar::new(
                                    k.to_string(),
                                    sg.suffix(),
                                    node_var.clone(),
                                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                );
                                visit_rel_delete_input::<RequestCtx>(
                                    Some(fragment.clone()),
                                    &rel_var,
                                    val,
                                    options.clone(),
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    sg,
                                    transaction,
                                    context,
                                )
                                .await?;
                            }
                        } else {
                            let rel_var = RelQueryVar::new(
                                k.to_string(),
                                sg.suffix(),
                                node_var.clone(),
                                NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                            );
                            visit_rel_delete_input::<RequestCtx>(
                                Some(fragment.clone()),
                                &rel_var,
                                v,
                                options.clone(),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                sg,
                                transaction,
                                context,
                            )
                            .await?;
                        }
                    }
                    _ => return Err(Error::TypeNotExpected { details: None }),
                }
            }
        }

        let result = transaction.delete_nodes(fragment, node_var, options).await;

        if let Some(handlers) = context
            .event_handlers()
            .after_node_delete(node_var.label()?)
        {
            for f in handlers.iter() {
                nodes = f(
                    nodes,
                    EventFacade::new(
                        CrudOperation::DeleteNode(node_var.label()?.to_string()),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }

        result
    })
}

async fn visit_node_input<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<QueryFragment, Error> {
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
                let node = visit_node_create_mutation_input::<RequestCtx>(
                    node_var,
                    v,
                    options,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    sg,
                    transaction,
                    context,
                )
                .await?;

                let mut id_props = HashMap::new();
                id_props.insert("id".to_string(), Comparison::default(node.id()?.clone()));

                Ok(transaction.node_read_fragment(Vec::new(), node_var, id_props, sg)?)
            }
            "EXISTING" => Ok(visit_node_query_input::<RequestCtx>(
                node_var,
                Some(v),
                options,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                sg,
                transaction,
            )
            .await?),
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + &*k,
            }),
        }
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

pub(crate) fn visit_node_query_input<'a, RequestCtx: RequestContext>(
    node_var: &'a NodeQueryVar,
    input: Option<Value>,
    options: Options,
    info: &'a Info,
    sg: &'a mut SuffixGenerator,
    transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
) -> BoxFuture<'a, Result<QueryFragment, Error>> {
    Box::pin(async move {
        trace!(
            "visit_node_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
            node_var,
            input,
            info.name()
        );

        let itd = info.type_def()?;
        let dst_var = NodeQueryVar::new(None, "dst".to_string(), sg.suffix());

        if let Some(Value::Map(m)) = input {
            let mut props = HashMap::new();
            let mut rqfs = Vec::new();
            for (k, v) in m.into_iter() {
                let p = itd.property(&k)?;
                match p.kind() {
                    PropertyKind::ScalarComp => {
                        props.insert(k, Comparison::try_from(v)?);
                    }
                    PropertyKind::Scalar => {
                        props.insert(k, Comparison::default(v));
                    }
                    PropertyKind::Input => {
                        rqfs.push(
                            visit_rel_query_input::<RequestCtx>(
                                None,
                                &RelQueryVar::new(
                                    k.to_string(),
                                    sg.suffix(),
                                    node_var.clone(),
                                    dst_var.clone(),
                                ),
                                Some(v),
                                options.clone(),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                sg,
                                transaction,
                            )
                            .await?,
                        );
                    }
                    _ => return Err(Error::TypeNotExpected { details: None }),
                }
            }

            transaction.node_read_fragment(rqfs, node_var, props, sg)
        } else {
            transaction.node_read_fragment(Vec::new(), node_var, HashMap::new(), sg)
        }
    })
}

pub(crate) async fn visit_node_update_input<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    mut input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error> {
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
        for f in handlers.iter() {
            input = f(
                input,
                EventFacade::new(
                    CrudOperation::UpdateNode(node_var.label()?.to_string()),
                    context,
                    transaction,
                    info,
                ),
            )
            .await?;
        }
        input
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let query_fragment = visit_node_query_input::<RequestCtx>(
            node_var,
            m.remove("MATCH"), // Remove used to take ownership
            options.clone(),
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
        )
        .await?;

        visit_node_update_mutation_input::<RequestCtx>(
            query_fragment,
            node_var,
            m.remove("SET").ok_or_else(|| {
                // remove() used here to take ownership of the "set" value, not borrow it
                Error::InputItemNotFound {
                    name: "input::SET".to_string(),
                }
            })?,
            options,
            &Info::new(
                itd.property("SET")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_update_mutation_input<'a, RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    node_var: &'a NodeQueryVar,
    input: Value,
    options: Options,
    info: &'a Info,
    sg: &'a mut SuffixGenerator,
    transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &'a GraphQLContext<RequestCtx>,
) -> BoxFuture<'a, Result<Vec<Node<RequestCtx>>, Error>> {
    Box::pin(async move {
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
                            validate_input(context.validators(), v_name, &input)
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
                        _ => return Err(Error::TypeNotExpected { details: None }),
                    }
                    Ok((props, inputs))
                },
            )?;

            let mut nodes = transaction
                .update_nodes(query_fragment, node_var, props, options.clone(), info, sg)
                .await?;

            if let Some(handlers) = context
                .event_handlers()
                .after_node_update(node_var.label()?)
            {
                for f in handlers.iter() {
                    nodes = f(
                        nodes,
                        EventFacade::new(
                            CrudOperation::UpdateNode(node_var.label()?.to_string()),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
            }

            if nodes.is_empty() {
                return Ok(nodes);
            }
            let node_fragment = transaction.node_read_by_ids_fragment(node_var, &nodes)?;

            for (k, v) in inputs.into_iter() {
                let p = itd.property(&k)?;

                match p.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => (), // Properties handled above
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            for val in input_array.into_iter() {
                                visit_rel_change_input::<RequestCtx>(
                                    node_fragment.clone(),
                                    &RelQueryVar::new(
                                        k.clone(),
                                        sg.suffix(),
                                        node_var.clone(),
                                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                    ),
                                    val,
                                    options.clone(),
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    sg,
                                    transaction,
                                    context,
                                )
                                .await?;
                            }
                        } else {
                            visit_rel_change_input::<RequestCtx>(
                                node_fragment.clone(),
                                &RelQueryVar::new(
                                    k,
                                    sg.suffix(),
                                    node_var.clone(),
                                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                ),
                                v,
                                options.clone(),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                sg,
                                transaction,
                                context,
                            )
                            .await?;
                        }
                    }
                    _ => return Err(Error::TypeNotExpected { details: None }),
                }
            }

            if let Some(handlers) = context
                .event_handlers()
                .after_node_subgraph_update(node_var.label()?)
            {
                for f in handlers.iter() {
                    nodes = f(
                        nodes,
                        EventFacade::new(
                            CrudOperation::UpdateNode(node_var.label()?.to_string()),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
            }

            Ok(nodes)
        } else {
            Err(Error::TypeNotExpected { details: None })
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_change_input<RequestCtx: RequestContext>(
    src_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<(), Error> {
    trace!(
        "visit_rel_change_input called -- src_fragment: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
        src_fragment, rel_var, input, info.name()
    );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input::<RequestCtx>(
                src_fragment,
                rel_var,
                v,
                options,
                &Info::new(
                    itd.property("ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;

            Ok(())
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input::<RequestCtx>(
                Some(src_fragment),
                rel_var,
                v,
                options,
                &Info::new(
                    itd.property("DELETE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;

            Ok(())
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<RequestCtx>(
                Some(src_fragment),
                rel_var,
                v,
                options,
                &Info::new(
                    itd.property("UPDATE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;
            Ok(())
        } else {
            Err(Error::InputItemNotFound {
                name: itd.type_name().to_string() + "::ADD|DELETE|UPDATE",
            })
        }
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn visit_rel_create_input<RequestCtx: RequestContext>(
    src_var: &NodeQueryVar,
    rel_name: &str,
    mut input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error> {
    trace!(
        "visit_rel_create_input called -- src_var: {:#?}, rel_name: {}, input: {:#?}, info.name: {}",
        src_var, rel_name, input, info.name()
    );

    let rel_label = src_var.label()?.to_string() + &*rel_name.to_string().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_create(&rel_label) {
        for f in handlers.iter() {
            input = f(
                input,
                EventFacade::new(
                    CrudOperation::CreateRel(src_var.label()?.to_string(), rel_name.to_string()),
                    context,
                    transaction,
                    info,
                ),
            )
            .await?;
        }
        input
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let src_fragment = visit_node_query_input::<RequestCtx>(
            src_var,
            m.remove("MATCH"), // Remove used to take ownership
            options.clone(),
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
        )
        .await?;

        let nodes = transaction
            .read_nodes::<RequestCtx>(src_var, src_fragment.clone(), options.clone(), info)
            .await?;

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
                visit_rel_create_mutation_input::<RequestCtx>(
                    src_fragment,
                    &rel_var,
                    create_input,
                    options,
                    &Info::new(
                        itd.property("CREATE")?.type_name().to_owned(),
                        info.type_defs(),
                    ),
                    sg,
                    transaction,
                    context,
                )
                .await
            }
            Value::Array(create_input_array) => {
                let mut rels = Vec::new();
                for create_input_value in create_input_array {
                    let rel_var = RelQueryVar::new(
                        rel_name.to_string(),
                        sg.suffix(),
                        src_var.clone(),
                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                    );
                    rels.append(
                        &mut visit_rel_create_mutation_input::<RequestCtx>(
                            src_fragment.clone(),
                            &rel_var,
                            create_input_value,
                            options.clone(),
                            &Info::new(
                                itd.property("CREATE")?.type_name().to_owned(),
                                info.type_defs(),
                            ),
                            sg,
                            transaction,
                            context,
                        )
                        .await?,
                    );
                }
                Ok(rels)
            }
            _ => Err(Error::TypeNotExpected { details: None }),
        }
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_create_mutation_input<RequestCtx: RequestContext>(
    src_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error> {
    trace!("visit_rel_create_mutation_input called -- src_fragment: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
            src_fragment, rel_var, input, info.name());

    if let Value::Map(mut m) = input {
        let dst_prop = info.type_def()?.property("dst")?;
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::InputItemNotFound {
                name: "dst".to_string(),
            })?;
        let dst_fragment = visit_rel_nodes_mutation_input_union::<RequestCtx>(
            rel_var.dst(),
            dst,
            options.clone(),
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await?;

        let rel_label =
            rel_var.src().label()?.to_string() + &*rel_var.label().to_title_case() + "Rel";
        let mut rels = transaction
            .create_rels(
                src_fragment,
                dst_fragment,
                rel_var,
                m.remove("id"),
                m,
                options,
                sg,
            )
            .await?;
        if let Some(handlers) = context.event_handlers().after_rel_create(&rel_label) {
            for f in handlers.iter() {
                rels = f(
                    rels,
                    EventFacade::new(
                        CrudOperation::CreateRel(
                            rel_var.src().label()?.to_string(),
                            rel_var.label().to_string(),
                        ),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }
        Ok(rels)
    } else {
        Err(Error::TypeNotExpected {
            details: Some("visit_rel_create_mutation_input input is not Map".to_string()),
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn visit_rel_delete_input<RequestCtx: RequestContext>(
    src_query_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    mut input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error> {
    trace!("visit_rel_delete_input called -- src_query_opt: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
    src_query_opt, rel_var, input, info.name());

    let rel_label = rel_var.src().label()?.to_string() + &*rel_var.label().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_delete(&rel_label) {
        for f in handlers.iter() {
            input = f(
                input,
                EventFacade::new(
                    CrudOperation::DeleteRel(
                        rel_var.src().label()?.to_string(),
                        rel_var.label().to_string(),
                    ),
                    context,
                    transaction,
                    info,
                ),
            )
            .await?;
        }
        input
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let fragment = visit_rel_query_input::<RequestCtx>(
            src_query_opt,
            rel_var,
            m.remove("MATCH"), // remove rather than get to take ownership
            options.clone(),
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
        )
        .await?;

        let rel_label =
            rel_var.src().label()?.to_string() + &*rel_var.label().to_title_case() + "Rel";
        let mut rels = transaction
            .read_rels(fragment, rel_var, options.clone())
            .await?;
        if rels.is_empty() {
            if let Some(handlers) = context.event_handlers().after_rel_delete(&rel_label) {
                let mut v = Vec::new();
                for f in handlers.iter() {
                    v = f(
                        v,
                        EventFacade::new(
                            CrudOperation::DeleteRel(
                                rel_var.src().label()?.to_string(),
                                rel_var.label().to_string(),
                            ),
                            context,
                            transaction,
                            info,
                        ),
                    )
                    .await?;
                }
            };
            return Ok(0);
        }

        let id_fragment = transaction.rel_read_by_ids_fragment(rel_var, &rels)?;

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input::<RequestCtx>(
                id_fragment.clone(),
                rel_var.src(),
                src,
                options.clone(),
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;
        }

        if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            visit_rel_dst_delete_mutation_input::<RequestCtx>(
                id_fragment.clone(),
                rel_var.dst(),
                dst,
                options.clone(),
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;
        }

        let result = transaction.delete_rels(id_fragment, rel_var, options).await;

        if let Some(handlers) = context.event_handlers().after_rel_delete(&rel_label) {
            for f in handlers.iter() {
                rels = f(
                    rels,
                    EventFacade::new(
                        CrudOperation::DeleteRel(
                            rel_var.src().label()?.to_string(),
                            rel_var.label().to_string(),
                        ),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }

        result
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_dst_delete_mutation_input<RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error> {
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

        visit_node_delete_mutation_input::<RequestCtx>(
            query_fragment,
            node_var,
            Some(v),
            options,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

async fn visit_rel_dst_query_input<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    input: Option<Value>,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
) -> Result<Option<QueryFragment>, Error> {
    trace!(
        "visit_rel_dst_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name()
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            Ok(Some(
                visit_node_query_input::<RequestCtx>(
                    node_var,
                    Some(v),
                    options,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    sg,
                    transaction,
                )
                .await?,
            ))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

async fn visit_rel_dst_update_mutation_input<RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error> {
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

        visit_node_update_mutation_input::<RequestCtx>(
            query_fragment,
            &NodeQueryVar::new(Some(k), "dst".to_string(), sg.suffix()),
            v,
            options,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

async fn visit_rel_nodes_mutation_input_union<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<QueryFragment, Error> {
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

        visit_node_input::<RequestCtx>(
            &NodeQueryVar::new(
                Some(k.clone()),
                node_var.base().to_string(),
                node_var.suffix().to_string(),
            ),
            v,
            options,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

pub(crate) async fn visit_rel_query_input<RequestCtx: RequestContext>(
    src_fragment_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    input_opt: Option<Value>,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
) -> Result<QueryFragment, Error> {
    trace!("visit_rel_query_input called -- src_fragment_opt: {:#?}, rel_var: {:#?}, input_opt: {:#?}, info.name(): {}",
        src_fragment_opt, rel_var, input_opt, info.name());

    let itd = info.type_def()?;
    let src_prop = itd.property("src")?;
    let dst_prop = itd.property("dst")?;

    if let Some(Value::Map(mut m)) = input_opt {
        // Remove used to take ownership
        let src_fragment_opt = if let Some(src) = m.remove("src") {
            visit_rel_src_query_input::<RequestCtx>(
                rel_var.src(),
                Some(src),
                options.clone(),
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                sg,
                transaction,
            )
            .await?
        } else {
            src_fragment_opt
        };

        // Remove used to take ownership
        let dst_query_opt = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input::<RequestCtx>(
                rel_var.dst(),
                Some(dst),
                options,
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                sg,
                transaction,
            )
            .await?
        } else {
            None
        };

        let mut value_props: HashMap<String, Comparison> = HashMap::new();
        for (k, v) in m.drain() {
            value_props.insert(k.to_string(), Comparison::try_from(v)?);
        }
        transaction.rel_read_fragment(src_fragment_opt, dst_query_opt, rel_var, value_props, sg)
    } else {
        transaction.rel_read_fragment(None, None, rel_var, HashMap::new(), sg)
    }
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_src_delete_mutation_input<RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<i32, Error> {
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

        visit_node_delete_mutation_input::<RequestCtx>(
            query_fragment,
            node_var,
            Some(v),
            options,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_src_update_mutation_input<RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    node_var: &NodeQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Node<RequestCtx>>, Error> {
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

        visit_node_update_mutation_input::<RequestCtx>(
            query_fragment,
            node_var,
            v,
            options,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            sg,
            transaction,
            context,
        )
        .await
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

async fn visit_rel_src_query_input<RequestCtx: RequestContext>(
    node_var: &NodeQueryVar,
    input: Option<Value>,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
) -> Result<Option<QueryFragment>, Error> {
    trace!(
        "visit_rel_src_query_input called -- node_var: {:#?}, input: {:#?}, info.name: {}",
        node_var,
        input,
        info.name(),
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let fragment = visit_node_query_input::<RequestCtx>(
                node_var,
                Some(v),
                options,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                sg,
                transaction,
            )
            .await?;

            Ok(Some(fragment))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn visit_rel_update_input<RequestCtx: RequestContext>(
    src_fragment_opt: Option<QueryFragment>,
    rel_var: &RelQueryVar,
    mut input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error> {
    trace!(
         "visit_rel_update_input called -- src_fragment_opt: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}",
         src_fragment_opt, rel_var, input, info.name());

    let rel_label = rel_var.src().label()?.to_string() + &*rel_var.label().to_title_case() + "Rel";
    let input = if let Some(handlers) = context.event_handlers().before_rel_update(&rel_label) {
        for f in handlers.iter() {
            input = f(
                input,
                EventFacade::new(
                    CrudOperation::UpdateRel(
                        rel_var.src().label()?.to_string(),
                        rel_var.label().to_string(),
                    ),
                    context,
                    transaction,
                    info,
                ),
            )
            .await?;
        }
        input
    } else {
        input
    };

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let fragment = visit_rel_query_input::<RequestCtx>(
            src_fragment_opt,
            rel_var,
            m.remove("MATCH"), // uses remove to take ownership
            options.clone(),
            &Info::new(
                itd.property("MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            sg,
            transaction,
        )
        .await?;

        trace!("visit_rel_update_input -- fragment: {:#?}", fragment);

        if let Some(update) = m.remove("SET") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<RequestCtx>(
                fragment,
                rel_var,
                update,
                options,
                &Info::new(
                    itd.property("SET")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await
        } else {
            Err(Error::InputItemNotFound {
                name: "input::SET".to_string(),
            })
        }
    } else {
        Err(Error::TypeNotExpected { details: None })
    }
}

#[allow(clippy::too_many_arguments)]
async fn visit_rel_update_mutation_input<RequestCtx: RequestContext>(
    query_fragment: QueryFragment,
    rel_var: &RelQueryVar,
    input: Value,
    options: Options,
    info: &Info,
    sg: &mut SuffixGenerator,
    transaction: &mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    context: &GraphQLContext<RequestCtx>,
) -> Result<Vec<Rel<RequestCtx>>, Error> {
    trace!(
         "visit_rel_update_mutation_input called -- query_fragment: {:#?}, rel_var: {:#?}: input: {:#?}, info.name: {}",
         query_fragment, rel_var, input, info.name());

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let src_opt = m.remove("src");
        let dst_opt = m.remove("dst");

        let rel_label =
            rel_var.src().label()?.to_string() + &*rel_var.label().to_title_case() + "Rel";
        let mut rels = transaction
            .update_rels(query_fragment, rel_var, m, options.clone(), sg)
            .await?;

        if let Some(handlers) = context.event_handlers().after_rel_update(&rel_label) {
            for f in handlers.iter() {
                rels = f(
                    rels,
                    EventFacade::new(
                        CrudOperation::UpdateRel(
                            rel_var.src().label()?.to_string(),
                            rel_var.label().to_string(),
                        ),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }
        if rels.is_empty() {
            return Ok(rels);
        }

        let id_fragment = transaction.rel_read_by_ids_fragment(rel_var, &rels)?;

        if let Some(src) = src_opt {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<RequestCtx>(
                id_fragment.clone(),
                rel_var.src(),
                src,
                options.clone(),
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;
        }

        if let Some(dst) = dst_opt {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input::<RequestCtx>(
                id_fragment,
                dst,
                options,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                sg,
                transaction,
                context,
            )
            .await?;
        }

        if let Some(handlers) = context
            .event_handlers()
            .after_rel_subgraph_update(&rel_label)
        {
            for f in handlers.iter() {
                rels = f(
                    rels,
                    EventFacade::new(
                        CrudOperation::UpdateRel(
                            rel_var.src().label()?.to_string(),
                            rel_var.label().to_string(),
                        ),
                        context,
                        transaction,
                        info,
                    ),
                )
                .await?;
            }
        }
        Ok(rels)
    } else {
        Err(Error::TypeNotExpected { details: None })
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
