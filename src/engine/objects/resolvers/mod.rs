use super::{Input, Node, Rel};
use crate::engine::context::{GlobalContext, GraphQLContext, RequestContext};
#[cfg(feature = "cosmos")]
use crate::engine::database::cosmos::CosmosTransaction;
#[cfg(feature = "neo4j")]
use crate::engine::database::neo4j::Neo4jTransaction;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use crate::engine::database::{ClauseType, Transaction};
use crate::engine::database::{DatabasePool, SuffixGenerator};
use crate::engine::resolvers::Object;
use crate::engine::resolvers::ResolverFacade;
use crate::engine::resolvers::{Arguments, ExecutionResult, Executor};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
#[cfg(feature = "neo4j")]
use tokio::runtime::Runtime;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input,
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

        trace!(
            "Resolver::resolve_node_create_mutation -- result: {:#?}",
            result
        );
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
        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let node_var = "node".to_string() + &sg.suffix();

        transaction.begin()?;
        let (query, params) = visit_node_create_mutation_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
            &node_var,
            &p.type_name(),
            ClauseType::Query,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &executor.context().validators(),
            transaction,
            &mut sg,
        )?;
        let results =
            transaction.create_node(query, params, &p.type_name(), self.partition_key_opt, info);

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

        trace!(
            "Resolver::resolve_node_delete_mutation -- results: {:#?}",
            results
        );

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
        let mut sg = SuffixGenerator::new();
        let itd = info
            .type_def()?
            .property(field_name)?
            .input_type_definition(info)?;
        let suffix = sg.suffix();

        transaction.begin()?;
        let (query, params) = visit_node_delete_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
            label,
            &("node".to_string() + &suffix),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            transaction,
            &mut sg,
        )?;
        let results = transaction.delete_nodes(query, params, label, self.partition_key_opt);

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

        trace!(
            "Resolver::resolve_node_read_query -- results: {:#?}",
            results
        );

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
        let mut sg = SuffixGenerator::new();

        let p = info.type_def()?.property(field_name)?;
        let itd = if info.name() == "Query" {
            p.input_type_definition(info)?
        } else {
            info.type_def_by_name("Query")?
                .property(p.type_name())?
                .input_type_definition(&info)?
        };
        let node_var = "node".to_string() + &sg.suffix();

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin()?;
        }
        let (match_fragment, where_fragment, params) = visit_node_query_input(
            HashMap::new(),
            p.type_name(),
            &node_var,
            true,
            false,
            ClauseType::Query,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input_opt.map(|i| i.value),
            transaction,
            &mut sg,
        )?;
        let (query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            p.type_name(),
            &node_var,
            true,
            false,
            ClauseType::Query,
            &sg.suffix(),
            HashMap::new(),
        )?;
        let results = transaction.read_nodes(query, self.partition_key_opt, Some(params), info);

        if info.name() == "Mutation" || info.name() == "Query" {
            if results.is_ok() {
                transaction.commit()?;
            } else {
                transaction.rollback()?;
            }
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

        trace!(
            "Resolver::resolve_node_update_mutation result: {:#?}",
            results
        );

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
        let mut sg = SuffixGenerator::new();
        let node_var = "node".to_string() + &sg.suffix();
        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        transaction.begin()?;
        let (query, params) = visit_node_update_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
            &node_var,
            &p.type_name(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &executor.context().validators(),
            transaction,
            &mut sg,
        )?;
        let result =
            transaction.update_nodes(query, params, &p.type_name(), self.partition_key_opt, info);

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
        let mut sg = SuffixGenerator::new();
        let validators = &executor.context().validators();

        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(p.type_name())?;

        transaction.begin()?;
        let (query, params) = visit_rel_create_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
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
            &mut sg,
        )?;
        let result = transaction.create_rels(
            query,
            params,
            src_label,
            Vec::new(),
            "",
            Vec::new(),
            rel_name,
            HashMap::new(),
            rtd.property("props").map(|pp| pp.type_name()).ok(),
            self.partition_key_opt,
            info,
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
        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let src_var = "src".to_string() + &sg.suffix();

        transaction.begin()?;
        let (query, params) = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
            &src_var,
            src_label,
            rel_name,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            transaction,
            &mut sg,
            true,
        )?;
        let results =
            transaction.delete_rels(query, params, src_label, rel_name, self.partition_key_opt);

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
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
        executor: &Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
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
        let mut sg = SuffixGenerator::new();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
        let _props_prop = rtd.property("props");
        let src_prop = rtd.property("src")?;
        let _dst_prop = rtd.property("dst")?;

        let src_suffix = sg.suffix();
        let rel_suffix = sg.suffix();
        let dst_suffix = sg.suffix();

        let src_var = "src".to_string() + &src_suffix;
        let dst_var = "dst".to_string() + &dst_suffix;

        if info.name() == "Mutation" || info.name() == "Query" {
            transaction.begin()?;
        }
        let (match_fragment, where_fragment, params) = visit_rel_query_input(
            HashMap::new(),
            &src_prop.type_name(),
            &src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            ClauseType::Query,
            true,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input_opt.map(|i| i.value),
            transaction,
            &mut sg,
        )?;
        let (query, params) = transaction.rel_read_query(
            &match_fragment,
            &where_fragment,
            params,
            src_prop.type_name(),
            &src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            true,
            ClauseType::Query,
            HashMap::new(),
            &mut sg,
        )?;
        let results = transaction.read_rels(
            query,
            Some(p.type_name()),
            self.partition_key_opt,
            Some(params),
        );

        if info.name() == "Mutation" || info.name() == "Query" {
            if results.is_ok() {
                transaction.commit()?;
            } else {
                transaction.rollback()?;
            }
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
        let mut sg = SuffixGenerator::new();
        let validators = &executor.context().validators();
        let td = info.type_def()?;
        let p = td.property(field_name)?;
        let itd = p.input_type_definition(info)?;
        let rtd = info.type_def_by_name(&p.type_name())?;
        let props_prop = rtd.property("props");
        let _src_prop = rtd.property("src")?;

        transaction.begin()?;
        let (query, params) = visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
            HashMap::new(),
            src_label,
            &("src".to_string() + &sg.suffix()),
            rel_name,
            true,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            validators,
            props_prop.map(|_| p.type_name()).ok(),
            transaction,
            &mut sg,
        )?;
        let results = transaction.update_rels(
            query,
            params,
            src_label,
            rel_name,
            Vec::new(),
            rtd.property("props").map(|_| p.type_name()).ok(),
            self.partition_key_opt,
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
        let mut sg = SuffixGenerator::new();

        match field_name {
            "dst" => {
                let node_var = "node".to_string() + &sg.suffix();
                let mut props = HashMap::new();
                props.insert("id".to_string(), dst_id.clone());
                let (match_fragment, where_fragment, params) = transaction.node_read_fragment(
                    Vec::new(),
                    HashMap::new(),
                    dst_label,
                    &node_var,
                    true,
                    false,
                    "",
                    props.clone(),
                    ClauseType::Query,
                )?;
                let (query, params) = transaction.node_read_query(
                    &match_fragment,
                    &where_fragment,
                    params,
                    dst_label,
                    &node_var,
                    true,
                    false,
                    ClauseType::Query,
                    "",
                    props,
                )?;
                transaction.read_nodes(query, self.partition_key_opt, Some(params), info)
            }
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + field_name,
            }),
        }
    }
}
