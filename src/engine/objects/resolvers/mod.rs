use crate::engine::context::{GraphQLContext, RequestContext};
use crate::engine::database::DatabasePool;
use crate::engine::database::{
    CrudOperation, NodeQueryVar, RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::events::EventFacade;
use crate::engine::loader::{NodeLoaderKey, RelLoaderKey};
use crate::engine::objects::Options;
use crate::engine::resolvers::Object;
use crate::engine::resolvers::ResolverFacade;
use crate::engine::resolvers::{Arguments, ExecutionResult, Executor};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use inflector::Inflector;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use ultra_batch::LoadError;
use visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input,
};

pub(crate) mod visitors;

pub(super) struct Resolver {}

impl Resolver {
    pub(super) fn new() -> Resolver {
        trace!("Resolver::new called");
        Resolver {}
    }

    #[tracing::instrument(
        level = "info",
        name = "execute_endpoint",
        skip(self, info, parent, args, executor)
    )]
    pub(super) async fn resolve_custom_endpoint<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        parent: Object<'_, RequestCtx>,
        args: &Arguments<'_>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_endpoint called -- info.name: {}, field_name: {}",
            info.name(),
            field_name,
        );

        // load resolver function
        let func = executor.context().resolver(field_name)?;

        // results
        func(ResolverFacade::new(
            field_name.to_string(),
            info,
            args,
            parent,
            executor,
        ))
        .await
    }

    #[tracing::instrument(
        level = "info",
        name = "resolve_custom_field",
        skip(self, info, resolver, parent, args, executor)
    )]
    pub(super) async fn resolve_custom_field<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        resolver: Option<&String>,
        parent: Object<'_, RequestCtx>,
        args: &Arguments<'_>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_field called -- info.name: {:#?}, field_name: {:#?}",
            info.name(),
            field_name,
        );

        let resolver_name = resolver.as_ref().ok_or_else(|| Error::ResolverNotFound {
            name: field_name.to_string(),
        })?;

        let func = &executor.context().resolver(resolver_name)?;

        func(ResolverFacade::new(
            field_name.to_string(),
            info,
            args,
            parent,
            executor,
        ))
        .await
    }

    #[tracing::instrument(
        level = "info",
        name = "resolve_custom_rel",
        skip(self, info, resolver, parent, args, executor)
    )]
    pub(super) async fn resolve_custom_rel<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        rel_name: &str,
        resolver: Option<&String>,
        parent: Object<'_, RequestCtx>,
        args: &Arguments<'_>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_rel called -- info.name: {}, rel_name: {}",
            info.name(),
            rel_name,
        );

        let resolver_name = resolver.as_ref().ok_or_else(|| Error::ResolverNotFound {
            name: rel_name.to_string(),
        })?;

        let func = &executor.context().resolver(resolver_name)?;

        func(ResolverFacade::new(
            rel_name.to_string(),
            info,
            args,
            parent,
            executor,
        ))
        .await
    }

    #[tracing::instrument(
        level = "info",
        name = "create_node",
        skip(self, info, input, executor)
    )]
    pub(super) async fn resolve_node_create_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_create_mutation called -- info.name: {}, field_name: {}, input: {:#?}, options: {:#?}",
            info.name(),
            field_name,
            input,
            options
        );

        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;
        let node_var = NodeQueryVar::new(
            Some(p.type_name().to_string()),
            "node".to_string(),
            sg.suffix(),
        );
        let results = visit_node_create_mutation_input::<RequestCtx>(
            &node_var,
            input,
            options,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        trace!(
            "Resolver::resolve_node_create_mutation -- result: {:#?}",
            results
        );
        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?,
            )
            .await
    }

    #[allow(unused_variables)]
    #[tracing::instrument(
        level = "info",
        name = "delete_node",
        skip(self, info, input, executor)
    )]
    pub(super) async fn resolve_node_delete_mutation<RequestCtx>(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult
    where
        RequestCtx: RequestContext,
    {
        trace!(
            "Resolver::resolve_node_delete_mutation called -- info.name: {}, field_name: {}: input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let mut sg = SuffixGenerator::new();
        let itd = info
            .type_def()?
            .property(field_name)?
            .input_type_definition(info)?;

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;

        let node_var = NodeQueryVar::new(Some(label.to_string()), "node".to_string(), sg.suffix());
        let results = visit_node_delete_input::<RequestCtx>(
            &node_var,
            input,
            options,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        trace!(
            "Resolver::resolve_node_delete_mutation -- results: {:#?}",
            results
        );

        executor.resolve_with_ctx(&(), &results?)
    }

    #[tracing::instrument(
        level = "info",
        name = "read_node",
        skip(self, info, input_opt, executor)
    )]
    pub(super) async fn resolve_node_read_query<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Value>,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_read_query called -- info.name: {}, field_name: {}, input_opt: {:#?}",
            info.name(),
            field_name,
            input_opt
        );

        let mut sg = SuffixGenerator::new();

        let p = info.type_def()?.property(field_name)?;
        let node_var = NodeQueryVar::new(
            Some(p.type_name().to_string()),
            "node".to_string(),
            sg.suffix(),
        );

        let mut transaction = executor.context().pool().read_transaction().await?;
        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin().await?;
        }

        let input_value_opt = if let Some(handlers) = executor
            .context()
            .event_handlers()
            .before_node_read(node_var.label()?)
        {
            let mut input_opt_value = input_opt;
            for f in handlers.iter() {
                input_opt_value = f(
                    input_opt_value,
                    EventFacade::new(
                        CrudOperation::ReadNode(field_name.to_string()),
                        executor.context(),
                        &mut transaction,
                        info,
                    ),
                )
                .await?;
            }
            input_opt_value
        } else {
            input_opt
        };

        let mut id_for_loader_opt = None;
        if options.sort().is_empty() {
            if let Some(Value::Map(im)) = &input_value_opt {
                if im.keys().len() == 1 {
                    if let Some(Value::Map(comparison)) = im.get("id") {
                        // Okay, this is painful, but we're testing whether after
                        // the possible additions of search criteria in the shape
                        // and the possible changes made to the input query by
                        // the before_node_read handler, we still have a query that
                        // has nothing in it but the node id search criterion.
                        // If so, this is a a basic node read (the most common case
                        // in a shape) and we should use the loader to avoid the
                        // N+1 problem.
                        //
                        // Unlike the rel loader, we don't handle the "IN" condition of multiple ids
                        // because errors aren't returned for each id being loaded, only one error if
                        // any of the operations fails. That means we can't return all found ids if
                        // even one id isn't found.
                        if let Some(id_val) = comparison.get("EQ") {
                            id_for_loader_opt =
                                Some(NodeLoaderKey::new(id_val.to_string(), options.clone()));
                        }
                    }
                }
            }
        }

        let mut results = if let Some(id_for_loader) = id_for_loader_opt {
            executor
                .context()
                .node_batcher()
                .load(id_for_loader)
                .await
                .map(|n| vec![n])
                .or_else(|e| {
                    if let LoadError::NotFound = e {
                        Ok(vec![])
                    } else {
                        Err(e)
                    }
                })?
        } else {
            let itd = if info.name() == "Query" {
                p.input_type_definition(info)?
            } else {
                info.type_def_by_name("Query")?
                    .property(p.type_name())?
                    .input_type_definition(info)?
            };
            let query_fragment = visit_node_query_input::<RequestCtx>(
                &node_var,
                input_value_opt,
                options.clone(),
                &Info::new(itd.type_name().to_owned(), info.type_defs()),
                &mut sg,
                &mut transaction,
            )
            .await?;

            match transaction
                .read_nodes(&node_var, query_fragment, options, info)
                .await
            {
                Err(e) => {
                    transaction.rollback().await?;
                    return Err(e.into());
                }
                Ok(results) => results,
            }
        };

        let label = match node_var.label() {
            Err(e) => {
                transaction.rollback().await?;
                return Err(e.into());
            }
            Ok(results) => results,
        };

        if let Some(handlers) = executor.context().event_handlers().after_node_read(label) {
            for f in handlers.iter() {
                results = match f(
                    results,
                    EventFacade::new(
                        CrudOperation::ReadNode(field_name.to_string()),
                        executor.context(),
                        &mut transaction,
                        info,
                    ),
                )
                .await
                {
                    Err(e) => {
                        transaction.rollback().await?;
                        return Err(e.into());
                    }
                    Ok(results) => results,
                }
            }
        }

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.commit().await?;
        }
        std::mem::drop(transaction);

        let type_name = results
            .get(0)
            .map(|n| n.type_name().to_string())
            .unwrap_or_else(|| p.type_name().to_string());

        if p.list() {
            executor
                .resolve_async(&Info::new(type_name, info.type_defs()), &results)
                .await
        } else {
            executor
                .resolve_async(&Info::new(type_name, info.type_defs()), &results.first())
                .await
        }
    }

    #[tracing::instrument(
        level = "info",
        name = "update_node",
        skip(self, info, input, executor)
    )]
    pub(super) async fn resolve_node_update_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );
        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;
        let results = visit_node_update_input::<RequestCtx>(
            &NodeQueryVar::new(
                Some(p.type_name().to_string()),
                "node".to_string(),
                sg.suffix(),
            ),
            input,
            options,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        trace!(
            "Resolver::resolve_node_update_mutation result: {:#?}",
            results
        );

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?,
            )
            .await
    }

    #[tracing::instrument(level = "info", name = "create_rel", skip(self, info, input, executor))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_create_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );

        let mut sg = SuffixGenerator::new();

        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let src_var =
            NodeQueryVar::new(Some(src_label.to_string()), "src".to_string(), sg.suffix());

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;
        let results = visit_rel_create_input::<RequestCtx>(
            &src_var,
            rel_name,
            input,
            options,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?,
            )
            .await
    }

    #[tracing::instrument(level = "info", name = "delete_rel", skip(self, info, input, executor))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_delete_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name, input
    );

        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        let rel_var = RelQueryVar::new(
            rel_name.to_string(),
            sg.suffix(),
            NodeQueryVar::new(Some(src_label.to_string()), "src".to_string(), sg.suffix()),
            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
        );

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;

        let results = visit_rel_delete_input::<RequestCtx>(
            None,
            &rel_var,
            input,
            options,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        executor.resolve_with_ctx(&(), &results?)
    }

    #[tracing::instrument(
        level = "info",
        name = "read_rel",
        skip(self, info, input_opt, executor)
    )]
    pub(super) async fn resolve_rel_read_query<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Value>,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, rel_name: {}, input_opt: {:#?}, options: {:#?}",
        info.name(),
        field_name,
        rel_name,
        input_opt,
        options
        );

        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(p.type_name())?;
        let src_prop = rtd.property("src")?;

        let dst_suffix = sg.suffix();
        let rel_suffix = sg.suffix();

        let src_var = NodeQueryVar::new(
            Some(src_prop.type_name().to_string()),
            "src".to_string(),
            sg.suffix(),
        );
        let dst_var = NodeQueryVar::new(None, "dst".to_string(), dst_suffix);
        let rel_var = RelQueryVar::new(rel_name.to_string(), rel_suffix, src_var, dst_var);

        let mut transaction = executor.context().pool().read_transaction().await?;
        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin().await?;
        }

        let input_value_opt = if let Some(handlers) =
            executor.context().event_handlers().before_rel_read(
                &(src_prop.type_name().to_string() + &*rel_var.label().to_title_case() + "Rel"),
            ) {
            let mut input_value_opt = input_opt;
            for f in handlers.iter() {
                input_value_opt = f(
                    input_value_opt,
                    EventFacade::new(
                        CrudOperation::ReadRel(field_name.to_string(), rel_name.to_string()),
                        executor.context(),
                        &mut transaction,
                        info,
                    ),
                )
                .await?;
            }
            input_value_opt
        } else {
            input_opt
        };

        let mut ids_for_loader_opt = None;
        if options.sort().is_empty() {
            if let Some(Value::Map(im)) = &input_value_opt {
                if im.keys().len() == 1 {
                    if let Some(Value::Map(src_m)) = im.get("src") {
                        if src_m.keys().len() == 1 {
                            if let Some(Value::Map(src_node_m)) =
                                src_m.get(info.type_def()?.type_name())
                            {
                                if src_node_m.keys().len() == 1 {
                                    if let Some(Value::Map(comparison)) = src_node_m.get("id") {
                                        // Okay, this is painful, but we're testing whether after
                                        // the possible additions of search criteria in the shape
                                        // and the possible changes made to the input dquery by
                                        // the before_rel_read handler, we still have a query that
                                        // has nothing in it but the src node id search criterion.
                                        // If so, this is a a basic rel read (the most common case
                                        // in a shape) and we should use the loader to avoid the
                                        // N+1 problem.
                                        if let Some(id_val) = comparison.get("EQ") {
                                            ids_for_loader_opt = Some(vec![RelLoaderKey::new(
                                                id_val.to_string(),
                                                rel_name.to_string(),
                                                options.clone(),
                                            )]);
                                        } else if let Some(Value::Array(ids)) = comparison.get("IN")
                                        {
                                            ids_for_loader_opt = Some(
                                                ids.iter()
                                                    .map(|id| {
                                                        Ok(RelLoaderKey::new(
                                                            id.to_string(),
                                                            rel_name.to_string(),
                                                            options.clone(),
                                                        ))
                                                    })
                                                    .collect::<Result<Vec<RelLoaderKey>, Error>>(
                                                    )?,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut results = if let Some(ids_for_loader) = ids_for_loader_opt {
            trace!("Resolver::resolve_rel_read_query about to call load.");
            executor
                .context()
                .rel_batcher()
                .load_many(&ids_for_loader)
                .await?
                .into_iter()
                .flatten()
                .collect()
        } else {
            let query_fragment = visit_rel_query_input::<RequestCtx>(
                None,
                &rel_var,
                input_value_opt,
                options.clone(),
                &Info::new(itd.type_name().to_owned(), info.type_defs()),
                &mut sg,
                &mut transaction,
            )
            .await?;

            transaction
                .read_rels(query_fragment, &rel_var, options)
                .await?
        };

        if let Some(handlers) = executor.context().event_handlers().after_rel_read(
            &(src_prop.type_name().to_string() + &*rel_var.label().to_title_case() + "Rel"),
        ) {
            for f in handlers.iter() {
                results = match f(
                    results,
                    EventFacade::new(
                        CrudOperation::ReadRel(field_name.to_string(), rel_name.to_string()),
                        executor.context(),
                        &mut transaction,
                        info,
                    ),
                )
                .await
                {
                    Err(e) => {
                        transaction.rollback().await?;
                        return Err(e.into());
                    }
                    Ok(results) => results,
                }
            }
        }

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.commit().await?;
        }
        std::mem::drop(transaction);

        if p.list() {
            executor
                .resolve_async(
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    &results,
                )
                .await
        } else {
            if results.len() > 1 {
                return Err(Error::RelDuplicated {
                    rel_name: rel_name.to_string(),
                    ids: results.iter().enumerate().try_fold(
                        String::new(),
                        |mut ids, (i, r)| -> Result<String, Error> {
                            if i > 0 {
                                ids.push_str(", ");
                            }
                            let id: String = r.id()?.clone().try_into()?;
                            ids.push_str(&id);
                            Ok(ids)
                        },
                    )?,
                }
                .into());
            }

            executor
                .resolve_async(
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    &results.first(),
                )
                .await
        }
    }

    #[tracing::instrument(level = "info", name = "update_rel", skip(self, info, input, executor))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_update_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Value,
        options: Options,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rel_var = RelQueryVar::new(
            rel_name.to_string(),
            sg.suffix(),
            NodeQueryVar::new(Some(src_label.to_string()), "src".to_string(), sg.suffix()),
            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
        );

        let mut transaction = executor.context().pool().transaction().await?;
        transaction.begin().await?;
        let results = visit_rel_update_input::<RequestCtx>(
            None,
            &rel_var,
            input,
            options.clone(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            &mut sg,
            &mut transaction,
            executor.context(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        std::mem::drop(transaction);

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?,
            )
            .await
    }

    pub(super) async fn resolve_scalar_field<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        fields: &HashMap<String, Value>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_scalar_field called -- info.name: {}, field_name: {}",
            info.name(),
            field_name,
        );

        fields.get(field_name).map_or_else(
            || {
                if field_name == "id" {
                    Err(Error::ResponseItemNotFound {
                        name: "id".to_string(),
                    }
                    .into())
                } else {
                    executor.resolve_with_ctx(&(), &None::<String>)
                }
            },
            |v| match v {
                Value::Null => executor.resolve_with_ctx(&(), &None::<String>),
                Value::Bool(_) => {
                    executor.resolve_with_ctx(&(), &TryInto::<bool>::try_into(v.clone())?)
                }
                Value::Int64(_) | Value::UInt64(_) => {
                    executor.resolve_with_ctx(&(), &TryInto::<i32>::try_into(v.clone())?)
                }
                Value::Float64(_) => {
                    executor.resolve_with_ctx(&(), &TryInto::<f64>::try_into(v.clone())?)
                }
                Value::String(_) => {
                    executor.resolve_with_ctx(&(), &TryInto::<String>::try_into(v.clone())?)
                }
                Value::Uuid(_) => {
                    executor.resolve_with_ctx(&(), &TryInto::<String>::try_into(v.clone())?)
                }
                Value::Array(a) => match a.get(0) {
                    Some(Value::Null) | Some(Value::String(_)) | Some(Value::Uuid(_)) => executor
                        .resolve_with_ctx(&(), &TryInto::<Vec<String>>::try_into(v.clone())?),
                    Some(Value::Bool(_)) => {
                        executor.resolve_with_ctx(&(), &TryInto::<Vec<bool>>::try_into(v.clone())?)
                    }
                    Some(Value::Int64(_)) | Some(Value::UInt64(_)) | Some(Value::Float64(_)) => {
                        let r = TryInto::<Vec<i32>>::try_into(v.clone());
                        if r.is_ok() {
                            executor.resolve_with_ctx(&(), &r?)
                        } else {
                            executor
                                .resolve_with_ctx(&(), &TryInto::<Vec<f64>>::try_into(v.clone())?)
                        }
                    }
                    Some(Value::Array(_)) | Some(Value::Map(_)) | None => {
                        Err((Error::TypeNotExpected {
                            details: Some(
                                "Expected Array of scalar, found Array of Array/Map".to_string(),
                            ),
                        })
                        .into())
                    }
                },
                Value::Map(_) => Err((Error::TypeNotExpected {
                    details: Some("Expected scalar, found Map".to_string()),
                })
                .into()),
            },
        )
    }

    pub(super) async fn resolve_static_version_query<RequestCtx: RequestContext>(
        &mut self,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        match &executor.context().version() {
            Some(v) => Ok(juniper::Value::scalar(v.to_string())),
            None => Ok(juniper::Value::Null),
        }
    }
}
