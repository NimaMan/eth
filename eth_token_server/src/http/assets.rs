use warp::{Filter, Rejection, Reply};

#[cfg(feature = "embed-static")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../../interface/asena/eth/tokens/dist"]
#[prefix = "eth/tokens/"]
struct TokenAssets;

#[cfg(feature = "embed-static")]
pub fn static_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("eth")
        .and(warp::path("tokens"))
        .and(warp::path::tail())
        .and_then(serve_embedded)
        .or(warp::path("eth")
            .and(warp::path("tokens"))
            .and_then(serve_index_embedded))
}

#[cfg(feature = "embed-static")]
async fn serve_embedded(tail: warp::path::Tail) -> Result<impl Reply, Rejection> {
    let path = tail.as_str();
    let path = if path.is_empty() { "index.html" } else { path };

    let (data, mime_type): (Vec<u8>, String) = match TokenAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (content.data.into_owned(), mime.to_string())
        }
        None => match TokenAssets::get("index.html") {
            Some(content) => (content.data.into_owned(), "text/html".to_string()),
            None => return Err(warp::reject::not_found()),
        },
    };

    Ok(warp::reply::with_header(data, "content-type", mime_type))
}

#[cfg(feature = "embed-static")]
async fn serve_index_embedded() -> Result<impl Reply, Rejection> {
    match TokenAssets::get("index.html") {
        Some(content) => Ok(warp::reply::with_header(
            content.data.into_owned(),
            "content-type",
            "text/html",
        )),
        None => Err(warp::reject::not_found()),
    }
}

#[cfg(not(feature = "embed-static"))]
pub fn static_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let static_files = warp::path("eth")
        .and(warp::path("tokens"))
        .and(warp::fs::dir("../../interface/asena/eth/tokens/dist"));

    let index_fallback = warp::path("eth")
        .and(warp::path("tokens"))
        .and(warp::fs::file("../../interface/asena/eth/tokens/dist/index.html"));

    static_files.or(index_fallback)
}
