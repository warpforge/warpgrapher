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
    executor: &Executor<GraphQLContext<crate::AppGlobalContext, crate::AppRequestContext>>,
    rf: Option<ReturnFunc<crate::AppGlobalContext, crate::AppRequestContext>>
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
