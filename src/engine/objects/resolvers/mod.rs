use super::Input;
use super::Node;
use crate::engine::context::{GlobalContext, GraphQLContext, RequestContext};
use crate::engine::database::Transaction;
use crate::engine::resolvers::Object;
use crate::engine::resolvers::ResolverFacade;
use crate::engine::resolvers::{Arguments, ExecutionResult, Executor};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use log::debug;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input, SuffixGenerator,
};

mod visitors;

pub(super) struct Resolver<'r, GlobalCtx, RequestCtx, T>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    partition_key_opt: Option<&'r Value>,
    executor: &'r Executor<'r, GraphQLContext<GlobalCtx, RequestCtx>>,
    transaction: &'r mut T,
}

impl<'r, GlobalCtx, RequestCtx, T> Resolver<'r, GlobalCtx, RequestCtx, T>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    pub(super) fn new(
        partition_key_opt: Option<&'r Value>,
        executor: &'r Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
        transaction: &'r mut T,
    ) -> Resolver<'r, GlobalCtx, RequestCtx, T> {
        trace!(
            "Resolver::new called -- partition_key_opt: {:#?}",
            partition_key_opt
        );
        Resolver {
            partition_key_opt,
            executor,
            transaction,
        }
    }

    pub(super) fn resolve_custom_endpoint(
        &mut self,
        info: &Info,
        field_name: &str,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_endpoint called -- info.name: {}, field_name: {}",
            info.name(),
            field_name,
        );

        // load resolver function
        let func = self.executor.context().resolver(field_name)?;

        // results
        func(ResolverFacade::new(
            field_name.to_string(),
            info,
            args,
            parent,
            self.partition_key_opt,
            self.executor,
        ))
    }

    pub(super) fn resolve_custom_field(
        &mut self,
        info: &Info,
        field_name: &str,
        resolver: Option<&String>,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_field called -- info.name: {:#?}, field_name: {:#?}",
            info.name(),
            field_name,
        );

        let resolver_name = resolver.as_ref().ok_or_else(|| Error::ResolverNotFound {
            name: field_name.to_string(),
        })?;

        let func = &self.executor.context().resolver(resolver_name)?;

        func(ResolverFacade::new(
            field_name.to_string(),
            info,
            args,
            parent,
            self.partition_key_opt,
            self.executor,
        ))
    }

    pub(super) fn resolve_custom_rel(
        &mut self,
        info: &Info,
        rel_name: &str,
        resolver: Option<&String>,
        parent: Object<GlobalCtx, RequestCtx>,
        args: &Arguments,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_custom_rel called -- info.name: {}, rel_name: {}",
            info.name(),
            rel_name,
        );

        let resolver_name = resolver.as_ref().ok_or_else(|| Error::ResolverNotFound {
            name: rel_name.to_string(),
        })?;

        let func = &self.executor.context().resolver(resolver_name)?;

        func(ResolverFacade::new(
            rel_name.to_string(),
            info,
            args,
            parent,
            self.partition_key_opt,
            self.executor,
        ))
    }

    pub(super) fn resolve_node_by_id(
        &mut self,
        label: &str,
        info: &Info,
        id: Value,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_by_id called -- label: {}, id: {:#?}",
            label,
            id
        );

        let mut props = HashMap::new();
        props.insert("id".to_string(), id);

        let (query, params) = self.transaction.node_query(
            Vec::new(),
            HashMap::new(),
            label,
            "0",
            false,
            true,
            "0",
            props,
        )?;
        let response =
            self.transaction
                .read_nodes(&query, self.partition_key_opt, Some(params), info)?;

        trace!("Resolver::resolve_node_by_id response: {:#?}", response);

        self.executor.resolve(
            &Info::new(label.to_string(), info.type_defs()),
            response.first().ok_or_else(|| Error::ResponseSetNotFound)?,
        )
    }

    pub(super) fn resolve_node_create_mutation(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_create_mutation called -- info.name: {}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        self.transaction.begin()?;
        let results = visit_node_create_mutation_input::<T, GlobalCtx, RequestCtx>(
            &p.type_name(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &self.executor.context().validators(),
            self.transaction,
        );
        trace!(
            "Resolver::resolve_node_create_mutation -- results: {:#?}",
            results
        );

        if results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        self.executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results?,
        )
    }

    pub(super) fn resolve_node_delete_mutation(
        &mut self,
        field_name: &str,
        label: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
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

        self.transaction.begin()?;
        let results = visit_node_delete_input::<T, GlobalCtx, RequestCtx>(
            label,
            &suffix,
            &mut sg,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            self.transaction,
        );
        trace!(
            "Resolver::resolve_node_delete_mutation -- results: {:#?}",
            results
        );

        if results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        self.executor.resolve_with_ctx(&(), &results?)
    }

    pub(super) fn resolve_node_read_query(
        &mut self,
        field_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
    ) -> ExecutionResult {
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

        self.transaction.begin()?;
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
            self.transaction,
        )?;
        let results =
            self.transaction
                .read_nodes(&query, self.partition_key_opt, Some(params), info);
        trace!(
            "Resolver::resolve_node_read_query -- results: {:#?}",
            results
        );

        if results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        if p.list() {
            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?,
            )
        } else {
            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results?.first(),
            )
        }
    }

    pub(super) fn resolve_node_update_mutation(
        &mut self,
        field_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
            info.name(),
            field_name,
            input
        );

        let p = info.type_def()?.property(field_name)?;
        let itd = p.input_type_definition(info)?;

        self.transaction.begin()?;
        let result = visit_node_update_input::<T, GlobalCtx, RequestCtx>(
            &p.type_name(),
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            self.partition_key_opt,
            input.value,
            &self.executor.context().validators(),
            self.transaction,
        );
        trace!(
            "Resolver::resolve_node_update_mutation result: {:#?}",
            result
        );

        if result.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        self.executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &result?,
        )
    }

    pub(super) fn resolve_rel_create_mutation(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );

        let validators = &self.executor.context().validators();

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
            self.transaction,
        );

        if result.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        let mutations = info.type_def_by_name("Mutation")?;
        let endpoint_td = mutations.property(field_name)?;

        if endpoint_td.list() {
            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &result?,
            )
        } else {
            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &result?[0],
            )
        }
    }

    pub(super) fn resolve_rel_delete_mutation(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
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
            self.transaction,
        );

        if results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        self.executor.resolve_with_ctx(&(), &results?)
    }

    pub(super) fn resolve_rel_props(
        &mut self,
        info: &Info,
        field_name: &str,
        props: &Node<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
        trace!(
            "Resolver::resolve_rel_props called -- info.name: {:#?}, field_name: {}",
            info.name(),
            field_name,
        );

        let td = info.type_def()?;
        let p = td.property(field_name)?;

        self.executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            props,
        )
    }

    pub(super) fn resolve_rel_read_query(
        &mut self,
        field_name: &str,
        src_ids_opt: Option<Vec<Value>>,
        rel_name: &str,
        info: &Info,
        input_opt: Option<Input<GlobalCtx, RequestCtx>>,
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
            self.transaction,
        )?;

        debug!(
            "Resolver::resolve_rel_read_query Query query, params: {} {:#?}",
            query, params
        );
        let raw_results = self.transaction.read_rels(
            &query,
            Some(p.type_name()),
            self.partition_key_opt,
            Some(params),
        );

        if raw_results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        let results = raw_results?;

        if p.list() {
            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results,
            )
        } else {
            if results.len() > 1 {
                return Err(Error::TypeNotExpected.into());
            }

            self.executor.resolve(
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                &results.first(),
            )
        }
    }

    pub(super) fn resolve_rel_update_mutation(
        &mut self,
        field_name: &str,
        src_label: &str,
        rel_name: &str,
        info: &Info,
        input: Input<GlobalCtx, RequestCtx>,
    ) -> ExecutionResult {
        trace!(
        "Resolver::resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

        let validators = &self.executor.context().validators();
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
            self.transaction,
        );

        if results.is_ok() {
            self.transaction.commit()?;
        } else {
            self.transaction.rollback()?;
        }

        self.executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results?,
        )
    }

    pub(super) fn resolve_scalar_field(
        &mut self,
        info: &Info,
        field_name: &str,
        fields: &HashMap<String, Value>,
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
                    self.executor.resolve_with_ctx(&(), &None::<String>)
                }
            },
            |v| match v {
                Value::Null => self.executor.resolve_with_ctx(&(), &None::<String>),
                Value::Bool(_) => self
                    .executor
                    .resolve_with_ctx(&(), &TryInto::<bool>::try_into(v.clone())?),
                Value::Int64(_) | Value::UInt64(_) => self
                    .executor
                    .resolve_with_ctx(&(), &TryInto::<i32>::try_into(v.clone())?),
                Value::Float64(_) => self
                    .executor
                    .resolve_with_ctx(&(), &TryInto::<f64>::try_into(v.clone())?),
                Value::String(_) => self
                    .executor
                    .resolve_with_ctx(&(), &TryInto::<String>::try_into(v.clone())?),
                Value::Array(a) => match a.get(0) {
                    Some(Value::Null) | Some(Value::String(_)) => self
                        .executor
                        .resolve_with_ctx(&(), &TryInto::<Vec<String>>::try_into(v.clone())?),
                    Some(Value::Bool(_)) => self
                        .executor
                        .resolve_with_ctx(&(), &TryInto::<Vec<bool>>::try_into(v.clone())?),
                    Some(Value::Int64(_)) | Some(Value::UInt64(_)) | Some(Value::Float64(_)) => {
                        let r = TryInto::<Vec<i32>>::try_into(v.clone());
                        if r.is_ok() {
                            self.executor.resolve_with_ctx(&(), &r?)
                        } else {
                            self.executor
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

    pub(super) fn resolve_static_version_query(&mut self) -> ExecutionResult {
        match &self.executor.context().version() {
            Some(v) => Ok(juniper::Value::scalar(v.to_string())),
            None => Ok(juniper::Value::Null),
        }
    }

    pub(super) fn resolve_union_field(
        &mut self,
        info: &Info,
        dst_label: &str,
        field_name: &str,
        dst_id: &Value,
    ) -> ExecutionResult {
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
                let (query, params) = self.transaction.node_query(
                    Vec::new(),
                    HashMap::new(),
                    dst_label,
                    "",
                    false,
                    true,
                    "",
                    props,
                )?;
                let results = self.transaction.read_nodes(
                    &query,
                    self.partition_key_opt,
                    Some(params),
                    info,
                )?;

                self.executor.resolve(
                    &Info::new(dst_label.to_string(), info.type_defs()),
                    &results.first().ok_or_else(|| Error::ResponseSetNotFound)?,
                )
            }
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + field_name,
            }
            .into()),
        }
    }
}