extern crate warpgrapher;
use self::warpgrapher::{Arguments, ExecutionResult, Executor, GraphQLContext, Info, Value};

pub fn resolver(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<crate::GlobalContext, crate::ReqContext>>,
) -> ExecutionResult {
    // extract global context
    let global_ctx = &executor.context().global_ctx;
    match global_ctx {
        Some(gctx) => {
            println!("gctx: {:#?}", gctx);
        }
        None => {}
    }

    // extract request context
    let req_ctx = &executor.context().req_ctx;
    match req_ctx {
        Some(rctx) => {
            println!("rctx: {:#?}", rctx);
        }
        None => {}
    }

    // get projects from database
    let graph = executor.context().pool.get().unwrap();
    let query = "MATCH (n:Project) RETURN (n);";
    let results = graph.exec(query).unwrap();

    // return number of projects
    let count = results.data.len();
    Ok(Value::scalar(count as i32))
}
