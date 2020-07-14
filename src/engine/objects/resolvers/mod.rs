use super::{Input, Node, Rel};
use crate::engine::context::{GlobalContext, GraphQLContext, RequestContext};
#[cfg(feature = "cosmos")]
use crate::engine::database::cosmos::CosmosTransaction;
#[cfg(feature = "neo4j")]
use crate::engine::database::neo4j::Neo4jTransaction;
use crate::engine::database::DatabasePool;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use crate::engine::database::Transaction;
use crate::engine::resolvers::Object;
use crate::engine::resolvers::ResolverFacade;
use crate::engine::resolvers::{Arguments, ExecutionResult, Executor};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use log::debug;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
#[cfg(feature = "neo4j")]
use tokio::runtime::Runtime;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input, SuffixGenerator,
};

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
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

    pub(super) fn resolve_custom_endpoint<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
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
    }

    pub(super) fn resolve_custom_field<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        resolver: Option<&String>,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
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
    }

    pub(super) fn resolve_custom_rel<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        rel_name: &str,
        resolver: Option<&String>,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
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
    }

    pub(super) fn resolve_node_by_id<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        info: &Info,
        id: Value,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_by_id called -- label: {}, id: {:#?}",
            label,
            id
        );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let results: Vec<Node<GlobalCtx, RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_node_by_id_with_transaction(
                label,
                info,
                id,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_node_by_id_with_transaction(
                    label,
                    info,
                    id,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve(
            &Info::new(label.to_string(), info.type_defs()),
            results.first().ok_or_else(|| Error::ResponseSetNotFound)?,
        )
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_node_by_id_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        label: &str,
        info: &Info,
        id: Value,
        transaction: &mut T,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
            "Resolver::resolve_node_by_id called -- label: {}, id: {:#?}",
            label,
            id
        );

        let mut props = HashMap::new();
        props.insert("id".to_string(), id);

        let (query, params) = transaction.node_query(
            Vec::new(),
            HashMap::new(),
            label,
            "0",
            false,
            true,
            "0",
            props,
        )?;
        transaction.read_nodes(&query, self.partition_key_opt, Some(params), info)
    }

    pub(super) fn resolve_node_create_mutation<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_create_mutation called -- info.name: {}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let result: Node<GlobalCtx, RequestCtx> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_node_create_mutation_with_transaction(
                field_name,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
                executor,
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_node_create_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                    executor,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &result,
        )
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_node_create_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
            "Resolver::resolve_node_create_mutation called -- info.name: {}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        transaction.begin()?;
        let results = visit_node_create_mutation_input::<T, GlobalCtx, RequestCtx>(
            &p.type_name(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &executor.context().validators(),
            transaction,
        );
        trace!(
            "Resolver::resolve_node_create_mutation -- results: {:#?}",
            results
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    #[allow(unused_variables)]
    pub(super) fn resolve_node_delete_mutation<GlobalCtx, RequestCtx>(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        trace!(
            "Resolver::resolve_node_delete_mutation called -- info.name: {}, field_name: {}: input: {:#?}",
            info.name(),
            field_name,
            input
        );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let results: i32 = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_node_delete_mutation_with_transaction(
                field_name,
                label,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_node_delete_mutation_with_transaction(
                    field_name,
                    label,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve_with_ctx(&(), &results)
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_node_delete_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
    ) -> Result<i32, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
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
        let suffix = sg.suffix();

        transaction.begin()?;
        let results = visit_node_delete_input::<T, GlobalCtx, RequestCtx>(
            label,
            &suffix,
            &mut sg,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            transaction,
        );
        trace!(
            "Resolver::resolve_node_delete_mutation -- results: {:#?}",
            results
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    pub(super) fn resolve_node_read_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_read_query called -- info.name: {}, field_name: {}, input_opt: {:#?}",
            info.name(),
            field_name,
            input_opt
        );
        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Node<GlobalCtx, RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_node_read_query_with_transaction(
                field_name,
                info,
                input_opt,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_node_read_query_with_transaction(
                    field_name,
                    info,
                    input_opt,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        if p.list() {
            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
        } else {
            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results.first(),
            )
        }
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_node_read_query_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
        transaction: &mut T,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
            "Resolver::resolve_node_read_query called -- info.name: {}, field_name: {}, input_opt: {:#?}",
            info.name(),
            field_name,
            input_opt
        );

        let mut sg = SuffixGenerator::new();

        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let suffix = sg.suffix();

        transaction.begin()?;
        let (query, params) = visit_node_query_input(
            &p.type_name(),
            &suffix,
            false,
            true,
            HashMap::new(),
            &mut sg,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input_opt.map(|i| i.value),
            transaction,
        )?;
        let results = transaction.read_nodes(&query, self.partition_key_opt, Some(params), info);
        trace!(
            "Resolver::resolve_node_read_query -- results: {:#?}",
            results
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    pub(super) fn resolve_node_update_mutation<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );
        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Node<GlobalCtx, RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_node_update_mutation_with_transaction(
                field_name,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
                executor,
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_node_update_mutation_with_transaction(
                    field_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                    executor,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results,
        )
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_node_update_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
            "Resolver::resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        transaction.begin()?;
        let result = visit_node_update_input::<T, GlobalCtx, RequestCtx>(
            &p.type_name(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &executor.context().validators(),
            transaction,
        );
        trace!(
            "Resolver::resolve_node_update_mutation result: {:#?}",
            result
        );

        if result.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        result
    }

    pub(super) fn resolve_rel_create_mutation<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );
        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let result: Vec<Rel<GlobalCtx, RequestCtx>> = match &executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_rel_create_mutation_with_transaction(
                field_name,
                src_label,
                rel_name,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
                executor,
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_rel_create_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                    executor,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        let mutations = info.type_def_by_name("Mutation")?;
        let endpoint_td = mutations.property(field_name)?;

        if endpoint_td.list() {
            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &result,
            )
        } else {
            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &result[0],
            )
        }
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_rel_create_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
        "Resolver::resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );

        let validators = &executor.context().validators();

        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(p.type_name())?;

        let result = visit_rel_create_input::<T, GlobalCtx, RequestCtx>(
            src_label,
            rel_name,
            // The conversion from Error to None using ok() is actually okay here,
            // as it's expected that some relationship types may not have props defined
            // in their schema, in which case the missing property is fine.
            rtd.property("props").map(|pp| pp.type_name()).ok(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            validators,
            transaction,
        );

        if result.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        result
    }

    pub(super) fn resolve_rel_delete_mutation<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name, input
    );
        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;

        let results: i32 = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_rel_delete_mutation_with_transaction(
                field_name,
                src_label,
                rel_name,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_rel_delete_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve_with_ctx(&(), &results)
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_rel_delete_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
    ) -> Result<i32, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
        "Resolver::resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name, input
    );

        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        let results = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
            src_label,
            None,
            rel_name,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            transaction,
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    pub(super) fn resolve_rel_props<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        props: &Node<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_rel_props called -- info.name: {:#?}, field_name: {}",
            info.name(),
            field_name,
        );

        let td = info.type_def()?;
        let p = td.property(field_name)?;

        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            props,
        )
    }

    pub(super) fn resolve_rel_read_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        field_name: &str,
        src_ids_opt: Option<Vec<Value>>,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, src_ids: {:#?}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
        src_ids_opt,
        rel_name,
        self.partition_key_opt,
        input_opt
    );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Rel<GlobalCtx, RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_rel_read_query_with_transaction(
                field_name,
                src_ids_opt,
                rel_name,
                info,
                input_opt,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_rel_read_query_with_transaction(
                    field_name,
                    src_ids_opt,
                    rel_name,
                    info,
                    input_opt,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        if p.list() {
            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
        } else {
            if results.len() > 1 {
                return Err(Error::TypeNotExpected.into());
            }

            executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results.first(),
            )
        }
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_rel_read_query_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_ids_opt: Option<Vec<Value>>,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
        transaction: &mut T,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
        "Resolver::resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, src_ids: {:#?}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
        src_ids_opt,
        rel_name,
        self.partition_key_opt,
        input_opt
    );

        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
        let _props_prop = rtd.property("props");
        let src_prop = rtd.property("src")?;
        let dst_prop = rtd.property("dst")?;

        let src_suffix = sg.suffix();
        let dst_suffix = sg.suffix();

        let (query, params) = visit_rel_query_input(
            &src_prop.type_name(),
            &src_suffix,
            src_ids_opt,
            rel_name,
            &dst_prop.type_name(),
            &dst_suffix,
            true,
            HashMap::new(),
            &mut sg,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input_opt.map(|i| i.value),
            transaction,
        )?;

        debug!(
            "Resolver::resolve_rel_read_query Query query, params: {} {:#?}",
            query, params
        );
        let results = transaction.read_rels(
            &query,
            Some(p.type_name()),
            self.partition_key_opt,
            Some(params),
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    pub(super) fn resolve_rel_update_mutation<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let p = info.type_def()?.property(field_name)?;

        let results: Vec<Rel<GlobalCtx, RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_rel_update_mutation_with_transaction(
                field_name,
                src_label,
                rel_name,
                info,
                input,
                &mut CosmosTransaction::new(c.clone()),
                executor,
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_rel_update_mutation_with_transaction(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                    executor,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results,
        )
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_rel_update_mutation_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
        transaction: &mut T,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
        "Resolver::resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

        let validators = &executor.context().validators();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
        let props_prop = rtd.property("props");
        let _src_prop = rtd.property("src")?;

        let results = visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
            src_label,
            None,
            rel_name,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            validators,
            props_prop.map(|_| p.type_name()).ok(),
            transaction,
        );

        if results.is_ok() {
            transaction.commit()?;
        } else {
            transaction.rollback()?;
        }

        results
    }

    pub(super) fn resolve_scalar_field<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        fields: &HashMap<String, Value>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
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
                Value::Array(a) => match a.get(0) {
                    Some(Value::Null) | Some(Value::String(_)) => executor
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

    pub(super) fn resolve_static_version_query<
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    >(
        &mut self,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        match &executor.context().version() {
            Some(v) => Ok(juniper::Value::scalar(v.to_string())),
            None => Ok(juniper::Value::Null),
        }
    }

    pub(super) fn resolve_union_field<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        dst_label: &str,
        field_name: &str,
        dst_id: &Value,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_union_field called -- info.name: {}, field_name: {}, dst_id: {:#?}",
            info.name(),
            field_name,
            dst_id
        );

        #[cfg(feature = "neo4j")]
        let mut runtime = Runtime::new()?;
        let results: Vec<Node<GlobalCtx, RequestCtx>> = match executor.context().pool() {
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => self.resolve_union_field_with_transaction(
                info,
                dst_label,
                field_name,
                dst_id,
                &mut CosmosTransaction::new(c.clone()),
            ),
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let c = runtime.block_on(p.get())?;
                self.resolve_union_field_with_transaction(
                    info,
                    dst_label,
                    field_name,
                    dst_id,
                    &mut Neo4jTransaction::new(c, &mut runtime),
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound),
        }?;

        executor.resolve(
            &Info::new(dst_label.to_string(), info.type_defs()),
            &results.first().ok_or_else(|| Error::ResponseSetNotFound)?,
        )
    }

    pub(super) fn resolve_union_field_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        info: &Info,
        field_name: &str,
        dst: &Node<GlobalCtx, RequestCtx>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!("Resolver::resolve_union_field_node called -- info.name: {}, field_name: {}, dst: {:#?}", info.name(), field_name, dst);

        executor.resolve(
            &Info::new(dst.type_name().to_string(), info.type_defs()),
            dst,
        )
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(super) fn resolve_union_field_with_transaction<GlobalCtx, RequestCtx, T>(
        &mut self,
        info: &Info,
        dst_label: &str,
        field_name: &str,
        dst_id: &Value,
        transaction: &mut T,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
        T: Transaction,
    {
        trace!(
            "Resolver::resolve_union_field called -- info.name: {}, field_name: {}, dst_id: {:#?}",
            info.name(),
            field_name,
            dst_id
        );

        match field_name {
            "dst" => {
                let mut props = HashMap::new();
                props.insert("id".to_string(), dst_id.clone());
                let (query, params) = transaction.node_query(
                    Vec::new(),
                    HashMap::new(),
                    dst_label,
                    "",
                    false,
                    true,
                    "",
                    props,
                )?;
                transaction.read_nodes(&query, self.partition_key_opt, Some(params), info)
            }
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + field_name,
            }),
        }
    }
}
