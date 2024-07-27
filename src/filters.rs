use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};
use warp::http::{HeaderMap, HeaderValue, Response, StatusCode};
use warp::hyper::Body;
use crate::handlers;
use crate::models::{ExportOptions, SharedAuthManager};

pub fn routes(manager: SharedAuthManager) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let mut headers = HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));

    preflight_options()
        .or(export())
        .or(poll_login(manager.clone()))
        .or(create_code(manager))

        .with(warp::reply::with::headers(headers))
}

fn with_manager(manager: SharedAuthManager) -> impl Filter<Extract = (SharedAuthManager,), Error = Infallible> + Clone {
    warp::any().map(move || manager.clone())
}

/// For CORS handling
pub fn preflight_options() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::any()
        .and(warp::options())
        .and_then(|| async move {
            Ok::<Response<Body>, Infallible>(Response::builder().status(StatusCode::NO_CONTENT)
                .header("Allow", "OPTIONS, GET, POST, DELETE")
                .header("Access-Control-Allow-Headers", "Authorization, Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .body("").unwrap().into_response())
        })
}

pub fn export() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("export")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::body::json::<ExportOptions>())
        .and_then(handlers::export_chunks)
}

pub fn create_code(manager: SharedAuthManager) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("code" / "create")
        .and(warp::get())
        .and(with_manager(manager))
        .and_then(handlers::create_code)
}

pub fn poll_login(manager: SharedAuthManager) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("code" / String / "poll")
        .and(warp::get())
        .and(with_manager(manager))
        .and_then(handlers::poll_login)
}
