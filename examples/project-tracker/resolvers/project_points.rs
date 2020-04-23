extern crate warpgrapher;
use self::warpgrapher::engine::objects::{Object};
use self::warpgrapher::engine::context::GraphQLContext;
use self::warpgrapher::engine::schema::Info;
use self::warpgrapher::juniper::{Arguments, ExecutionResult, Executor, Value};
use warpgrapher::engine::resolvers::{ReturnFunc};

pub fn resolver(
    _info: &Info,
    _field_name: &str,
    _parent: Object<crate::AppGlobalContext, crate::AppRequestContext>,
    _args: &Arguments,
    _executor: &Executor<GraphQLContext<crate::AppGlobalContext, crate::AppRequestContext>>,
    _rf: Option<ReturnFunc<crate::AppGlobalContext, crate::AppRequestContext>>
) -> ExecutionResult {
    Ok(Value::scalar(1_000_000 as i32))
}
