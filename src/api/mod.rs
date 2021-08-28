use std::{convert::Infallible, error::Error};

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

/// Convert rejections into a JSON response.
#[allow(clippy::unused_async)]
pub async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
    let (reason, status) = if err.is_not_found() {
        ("Not Found", StatusCode::NOT_FOUND)
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        if let Some(cause) = e.source() {
            tracing::debug!("deserialize error: {:?}", cause);
            if let Some(err) = cause.downcast_ref::<serde_json::Error>() {
                return Ok(json_error_response(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }
        }
        ("Bad Request", StatusCode::BAD_REQUEST)
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        ("Payload Too Large", StatusCode::PAYLOAD_TOO_LARGE)
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        ("Method Not Allowed", StatusCode::METHOD_NOT_ALLOWED)
    } else {
        tracing::warn!("unhandled rejection: {:?}", err);
        ("Internal Server Error", StatusCode::INTERNAL_SERVER_ERROR)
    };

    Ok(json_error_response(reason, status))
}

#[derive(serde::Serialize)]
struct ErrorMessage {
    reason: String,
}

fn json_error_response<T: Into<String>>(
    reason: T,
    status: warp::http::StatusCode,
) -> reply::Response {
    reply::with_status(
        reply::json(&ErrorMessage {
            reason: reason.into(),
        }),
        status,
    )
    .into_response()
}
