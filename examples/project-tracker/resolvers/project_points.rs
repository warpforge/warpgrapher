extern crate warpgrapher;
use self::warpgrapher::engine::context::GraphQLContext;
use self::warpgrapher::engine::schema::Info;
use self::warpgrapher::juniper::{Arguments, ExecutionResult, Executor, Value};

pub fn resolver(
    _info: &Info,
    _args: &Arguments,
    _executor: &Executor<GraphQLContext<crate::GlobalContext, crate::ReqContext>>,
) -> ExecutionResult {
    Ok(Value::scalar(1_000_000 as i32))
}
