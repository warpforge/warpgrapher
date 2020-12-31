use super::{Input, Node, Rel};
use crate::engine::context::{GraphQLContext, RequestContext};
#[cfg(any(feature = "cosmos", feature = "gremlin"))]
use crate::engine::database::gremlin::GremlinTransaction;
#[cfg(feature = "neo4j")]
use crate::engine::database::neo4j::Neo4jTransaction;
use crate::engine::database::DatabasePool;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use crate::engine::database::{NodeQueryVar, RelQueryVar, SuffixGenerator, Transaction};
use crate::engine::resolvers::Object;
use crate::engine::resolvers::ResolverFacade;
use crate::engine::resolvers::{Arguments, ExecutionResult, Executor};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input,
};

#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
mod visitors;

pub(super) struct Resolver<'r> {
    partition_key_opt: Option<&'r Value>,
}

impl<'r> Resolver<'r> {
    pub(super) fn new(partition_key_opt: Option<&'r Value>) -> Resolver<'r> {
        trace!(
            "Resolver::new called -- partition_key_opt: {:#?}",
            partition_key_opt
        );
        Resolver { partition_key_opt }
    }

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
            self.partition_key_opt,
            executor,
        ))
        .await
    }

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
            self.partition_key_opt,
            &executor,
        ))
        .await
    }

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
            self.partition_key_opt,
            executor,
        ))
        .await
    }

    pub(super) async fn resolve_node_create_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_create_mutation called -- info.name: {}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let p = info.type_def()?.property(field_name)?;

        let result: Node<RequestCtx> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_node_create_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                    executor,
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_node_create_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                    executor,
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_node_create_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                    executor,
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        trace!(
            "Resolver::resolve_node_create_mutation -- result: {:#?}",
            result
        );
        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &result,
            )
            .await
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_node_create_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> Result<Node<RequestCtx>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        transaction.begin().await?;
        let node_var = NodeQueryVar::new(
            Some(p.type_name().to_string()),
            "node".to_string(),
            sg.suffix(),
        );
        let results = visit_node_create_mutation_input::<T, RequestCtx>(
            &node_var,
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
            &executor.context().validators(),
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }

        results
    }

    #[allow(unused_variables)]
    pub(super) async fn resolve_node_delete_mutation<RequestCtx>(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Input<RequestCtx>,
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

        let results: i32 = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_node_delete_mutation_with_transaction(
                    field_name,
                    label,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_node_delete_mutation_with_transaction(
                    field_name,
                    label,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_node_delete_mutation_with_transaction(
                    field_name,
                    label,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        trace!(
            "Resolver::resolve_node_delete_mutation -- results: {:#?}",
            results
        );

        executor.resolve_with_ctx(&(), &results)
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_node_delete_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
    ) -> Result<i32, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let itd = info
            .type_def()?
            .property(field_name)?
            .input_type_definition(info)?;

        transaction.begin().await?;

        let node_var = NodeQueryVar::new(Some(label.to_string()), "node".to_string(), sg.suffix());
        let results = visit_node_delete_input::<T, RequestCtx>(
            &node_var,
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }

        results
    }

    pub(super) async fn resolve_node_read_query<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Input<RequestCtx>>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_read_query called -- info.name: {}, field_name: {}, input_opt: {:#?}",
            info.name(),
            field_name,
            input_opt
        );
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Node<RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_node_read_query_with_transaction(
                    field_name,
                    info,
                    input_opt,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_node_read_query_with_transaction(
                    field_name,
                    info,
                    input_opt,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_node_read_query_with_transaction(
                    field_name,
                    info,
                    input_opt,
                    &mut Neo4jTransaction::new(c),
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        trace!(
            "Resolver::resolve_node_read_query -- results: {:#?}",
            results
        );

        if p.list() {
            executor
                .resolve_async(
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    &results,
                )
                .await
        } else {
            executor
                .resolve_async(
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    &results.first(),
                )
                .await
        }
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_node_read_query_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Input<RequestCtx>>,
        transaction: &mut T,
    ) -> Result<Vec<Node<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();

        let p = info.type_def()?.property(field_name)?;
        let itd = if info.name() == "Query" {
            p.input_type_definition(info)?
        } else {
            info.type_def_by_name("Query")?
                .property(p.type_name())?
                .input_type_definition(&info)?
        };
        let node_var = NodeQueryVar::new(
            Some(p.type_name().to_string()),
            "node".to_string(),
            sg.suffix(),
        );

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin().await?;
        }

        let query_fragment = visit_node_query_input(
            &node_var,
            input_opt.map(|i| i.value),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
        )
        .await?;

        let results = transaction
            .read_nodes(&node_var, query_fragment, self.partition_key_opt, info)
            .await;

        if info.name() == "Mutation" || info.name() == "Query" {
            if results.is_ok() {
                transaction.commit().await?;
            } else {
                transaction.rollback().await?;
            }
        }

        results
    }

    pub(super) async fn resolve_node_update_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Node<RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_node_update_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                    executor,
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_node_update_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                    executor,
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_node_update_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                    executor,
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        trace!(
            "Resolver::resolve_node_update_mutation result: {:#?}",
            results
        );

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
            .await
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_node_update_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> Result<Vec<Node<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        transaction.begin().await?;
        let result = visit_node_update_input::<T, RequestCtx>(
            &NodeQueryVar::new(
                Some(p.type_name().to_string()),
                "node".to_string(),
                sg.suffix(),
            ),
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
            &executor.context().validators(),
        )
        .await;

        if result.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }
        result
    }

    pub(super) async fn resolve_rel_create_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Rel<RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_rel_create_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                    executor,
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_rel_create_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                    executor,
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_rel_create_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                    executor,
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
            .await
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_create_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let validators = &executor.context().validators();

        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(p.type_name())?;
        let src_var =
            NodeQueryVar::new(Some(src_label.to_string()), "src".to_string(), sg.suffix());

        transaction.begin().await?;
        let result = visit_rel_create_input::<T, RequestCtx>(
            &src_var,
            rel_name,
            // The conversion from Error to None using ok() is actually okay here,
            // as it's expected that some relationship types may not have props defined
            // in their schema, in which case the missing property is fine.
            rtd.property("props").map(|pp| pp.type_name()).ok(),
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
            validators,
        )
        .await;

        if result.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }

        result
    }

    pub(super) async fn resolve_rel_delete_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name, input
    );

        let results: i32 = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_rel_delete_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_rel_delete_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_rel_delete_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve_with_ctx(&(), &results)
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_rel_delete_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
    ) -> Result<i32, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
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

        transaction.begin().await?;

        let results = visit_rel_delete_input::<T, RequestCtx>(
            None,
            &rel_var,
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }

        results
    }

    pub(super) async fn resolve_rel_props<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        props: &Node<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_rel_props called -- info.name: {:#?}, field_name: {}",
            info.name(),
            field_name,
        );

        let td = info.type_def()?;
        let p = td.property(field_name)?;

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                props,
            )
            .await
    }

    pub(super) async fn resolve_rel_read_query<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<RequestCtx>>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
        rel_name,
        self.partition_key_opt,
        input_opt
    );

        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Rel<RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_rel_read_query_with_transaction(
                    field_name,
                    rel_name,
                    info,
                    input_opt,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_rel_read_query_with_transaction(
                    field_name,
                    rel_name,
                    info,
                    input_opt,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_rel_read_query_with_transaction(
                    field_name,
                    rel_name,
                    info,
                    input_opt,
                    &mut Neo4jTransaction::new(c),
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

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
                            let id: String = r.id().clone().try_into()?;
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

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_read_query_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<RequestCtx>>,
        transaction: &mut T,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
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

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin().await?;
        }
        let query_fragment = visit_rel_query_input(
            None,
            &rel_var,
            input_opt.map(|i| i.value),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
        )
        .await?;
        let results = transaction
            .read_rels(
                query_fragment,
                &rel_var,
                Some(p.type_name()),
                self.partition_key_opt,
            )
            .await;

        if info.name() == "Mutation" || info.name() == "Query" {
            if results.is_ok() {
                transaction.commit().await?;
            } else {
                transaction.rollback().await?;
            }
        }

        results
    }

    pub(super) async fn resolve_rel_update_mutation<RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Rel<RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_rel_update_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                    executor,
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_rel_update_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                    executor,
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_rel_update_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c),
                    executor,
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor
            .resolve_async(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
            .await
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resolve_rel_update_mutation_with_transaction<RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<RequestCtx>,
        transaction: &mut T,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();
        let validators = &executor.context().validators();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
        let props_prop = rtd.property("props");
        let rel_var = RelQueryVar::new(
            rel_name.to_string(),
            sg.suffix(),
            NodeQueryVar::new(Some(src_label.to_string()), "src".to_string(), sg.suffix()),
            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
        );

        transaction.begin().await?;
        let results = visit_rel_update_input::<T, RequestCtx>(
            None,
            &rel_var,
            props_prop.map(|_| p.type_name()).ok(),
            input.value,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            &mut sg,
            transaction,
            validators,
        )
        .await;

        if results.is_ok() {
            transaction.commit().await?;
        } else {
            transaction.rollback().await?;
        }

        results
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
                        Err(Error::TypeNotExpected.into())
                    }
                },
                Value::Map(_) => Err(Error::TypeNotExpected.into()),
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

    pub(super) async fn resolve_union_field<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        dst_label: &str,
        field_name: &str,
        dst_id: &Value,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_union_field called -- info.name: {}, field_name: {}, dst_id: {:#?}",
            info.name(),
            field_name,
            dst_id
        );

        let results: Vec<Node<RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                self.resolve_union_field_with_transaction(
                    info,
                    dst_label,
                    field_name,
                    dst_id,
                    &mut GremlinTransaction::new(c.clone(), true, false),
                )
                .await
            }
            #[cfg(feature = "gremlin")]
            DatabasePool::Gremlin((c, uuid)) => {
                self.resolve_union_field_with_transaction(
                    info,
                    dst_label,
                    field_name,
                    dst_id,
                    &mut GremlinTransaction::new(c.clone(), false, *uuid),
                )
                .await
            }
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = p.get().await?;
                self.resolve_union_field_with_transaction(
                    info,
                    dst_label,
                    field_name,
                    dst_id,
                    &mut Neo4jTransaction::new(c),
                )
                .await
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor
            .resolve_async(
                &Info::new(dst_label.to_string(), info.type_defs()),
                &results.first().ok_or(Error::ResponseSetNotFound)?,
            )
            .await
    }

    pub(super) async fn resolve_union_field_node<RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        dst: &Node<RequestCtx>,
        executor: &Executor<'_, '_, GraphQLContext<RequestCtx>>,
    ) -> ExecutionResult {
        trace!("Resolver::resolve_union_field_node called -- info.name: {}, field_name: {}, dst: {:#?}", info.name(), field_name, dst);

        executor
            .resolve_async(
                &Info::new(dst.type_name().to_string(), info.type_defs()),
                dst,
            )
            .await
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(super) async fn resolve_union_field_with_transaction<RequestCtx, T>(
        &mut self,
        info: &Info,
        dst_label: &str,
        field_name: &str,
        dst_id: &Value,
        transaction: &mut T,
    ) -> Result<Vec<Node<RequestCtx>>, Error>
    where
        RequestCtx: RequestContext,
        T: Transaction,
    {
        let mut sg = SuffixGenerator::new();

        match field_name {
            "dst" => {
                let node_var =
                    NodeQueryVar::new(Some(dst_label.to_string()), "node".to_string(), sg.suffix());
                let mut props = HashMap::new();
                props.insert("id".to_string(), dst_id.clone());
                let query_fragment =
                    transaction.node_read_fragment(Vec::new(), &node_var, props, &mut sg)?;
                transaction
                    .read_nodes(&node_var, query_fragment, self.partition_key_opt, info)
                    .await
            }
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + field_name,
            }),
        }
    }
}
