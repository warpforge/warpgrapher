extern crate warpgrapher;
use self::warpgrapher::{Arguments, ExecutionResult, Executor, GraphQLContext, Info, Value};

pub fn resolver(
    _info: &Info,
    _args: &Arguments,
    _executor: &Executor<GraphQLContext<crate::GlobalContext, crate::ReqContext>>,
) -> ExecutionResult {
    Ok(Value::scalar(1_000_000 as i32))
}
