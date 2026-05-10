#[cfg(not(feature = "embed-static"))]
use std::path::PathBuf;
use warp::{Filter, Rejection, Reply};

#[cfg(feature = "embed-static")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../../interface/asena/eth/tokens/dist"]
#[prefix = "eth/tokens/"]
struct TokenAssets;

/// Resolve a URL tail path to the correct static file path.
/// Handles multi-page HTML fallbacks for token surface routes.
fn resolve_path(tail: &str) -> String {
    let tail = tail.trim_start_matches('/').trim_end_matches('/');

    // Exact file matches (assets, html files, etc.)
    if tail.contains('.') || tail.starts_with("assets/") {
        return tail.to_string();
    }

    // Known page routes
    match tail {
        "" | "index.html" => return "index.html".to_string(),
        "live" | "live/index.html" => return "live.html".to_string(),
        "signals" | "signals/index.html" => return "signals.html".to_string(),
        "cache" | "cache/index.html" => return "cache.html".to_string(),
        _ => {}
    }

    // Live detail: live/<address>/
    if let Some(rest) = tail.strip_prefix("live/") {
        let addr_part = rest.trim_end_matches("/index.html");
        if is_token_address(addr_part) {
            return "live-detail.html".to_string();
        }
    }

    // Detail: <address>/
    let addr_part = tail.trim_end_matches("/index.html");
    if is_token_address(addr_part) {
        return "detail.html".to_string();
    }

    tail.to_string()
}

fn is_token_address(s: &str) -> bool {
    s.len() == 42
        && s.starts_with("0x")
        && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(feature = "embed-static")]
pub fn static_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("eth")
        .and(warp::path("tokens"))
        .and(warp::path::tail())
        .and_then(serve_embedded)
}

#[cfg(feature = "embed-static")]
async fn serve_embedded(tail: warp::path::Tail) -> Result<impl Reply, Rejection> {
    let path = resolve_path(tail.as_str());

    // Try exact file first
    if let Some(content) = TokenAssets::get(&path) {
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        return Ok(warp::reply::with_header(
            content.data.into_owned(),
            "content-type",
            mime.to_string(),
        ));
    }

    // Fall back to index.html for unmatched HTML routes
    if path.ends_with(".html") {
        if let Some(content) = TokenAssets::get("index.html") {
            return Ok(warp::reply::with_header(
                content.data.into_owned(),
                "content-type",
                "text/html",
            ));
        }
    }

    Err(warp::reject::not_found())
}

#[cfg(not(feature = "embed-static"))]
pub fn static_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let dist_dir: PathBuf = "../../interface/asena/eth/tokens/dist".into();

    warp::path("eth")
        .and(warp::path("tokens"))
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let dist_dir = dist_dir.clone();
            async move { serve_from_disk(dist_dir, tail.as_str()).await }
        })
}

#[cfg(not(feature = "embed-static"))]
async fn serve_from_disk(dist_dir: PathBuf, tail: &str) -> Result<impl Reply, Rejection> {
    let path = resolve_path(tail);
    let file_path = dist_dir.join(&path);

    // Try exact file first
    match tokio::fs::read(&file_path).await {
        Ok(data) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            return Ok(warp::reply::with_header(data, "content-type", mime.to_string()));
        }
        Err(_) => {
            // Fall back to index.html for unmatched HTML routes
            if path.ends_with(".html") {
                let index_path = dist_dir.join("index.html");
                if let Ok(data) = tokio::fs::read(index_path).await {
                    return Ok(warp::reply::with_header(data, "content-type", "text/html"));
                }
            }
        }
    }

    Err(warp::reject::not_found())
}
