use std::convert::Infallible;

use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

pub mod files;
pub mod proxy;

fn with_context<T>(ctx: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone
where
    T: Clone + Send,
{
    warp::any().map(move || ctx.clone())
}

fn json_body<T>() -> impl Filter<Extract = (T,), Error = Rejection> + Clone
where
    T: serde::de::DeserializeOwned + Send,
{
    warp::body::content_length_limit(2 * 1024 * 1024).and(warp::body::json())
}

fn json_response<T: serde::Serialize>(res: &T, status: StatusCode) -> reply::Response {
    reply::with_status(reply::json(res), status).into_response()
}
