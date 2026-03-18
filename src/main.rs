use std::env;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper::header::HeaderMap;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::net::TcpListener;

const DEFAULT_BASE_URL: &str = "http://100.102.55.49:3000";
const DEFAULT_PORT: u16 = 8080;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Parse the incoming path into the Dokploy deploy API path.
/// - "/<token>" -> "/api/deploy/<token>"
/// - "/compose/<token>" -> "/api/deploy/compose/<token>"
/// Returns None if the path doesn't match.
fn map_path(path: &str) -> Option<String> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    if let Some(token) = trimmed.strip_prefix("compose/") {
        if !token.is_empty() && !token.contains('/') {
            return Some(format!("/api/deploy/compose/{}", token));
        }
        return None;
    }

    if !trimmed.contains('/') {
        return Some(format!("/api/deploy/{}", trimmed));
    }

    None
}

/// Check if the Content-Type header indicates form-urlencoded data.
fn content_type_is_form_urlencoded(headers: &HeaderMap) -> bool {
    headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false)
}

/// Extract JSON from a form-urlencoded body.
/// GitHub sends webhooks as `payload=<url-encoded-json>` when the webhook
/// content type is set to `application/x-www-form-urlencoded`.
fn extract_json_from_form(body: &Bytes) -> Option<Bytes> {
    for (key, value) in form_urlencoded::parse(body) {
        if key == "payload" {
            return Some(Bytes::from(value.into_owned()));
        }
    }
    None
}

async fn handle(
    base_url: String,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, BoxError> {
    // Only allow POST
    if req.method() != Method::POST {
        eprintln!("{} {} -> 405 Method Not Allowed", req.method(), req.uri().path());
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from("Method Not Allowed\n")))
            .unwrap());
    }

    let path = req.uri().path().to_string();

    // Map the path
    let upstream_path = match map_path(&path) {
        Some(p) => p,
        None => {
            eprintln!("POST {} -> 404 Not Found", path);
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found\n")))
                .unwrap());
        }
    };

    let upstream_uri: Uri = format!("{}{}", base_url, upstream_path).parse()?;
    eprintln!("POST {} -> forwarding to {}", path, upstream_uri);

    // Build upstream request preserving headers and body
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(&upstream_uri);

    // Extract headers before consuming the request body
    let headers = req.headers().clone();

    // Copy headers (skip Host, it should match the upstream)
    for (name, value) in &headers {
        if name != hyper::header::HOST {
            builder = builder.header(name, value);
        }
    }

    // Collect the incoming body
    let body_bytes = req.into_body().collect().await?.to_bytes();

    // If the body is application/x-www-form-urlencoded (GitHub default), extract the
    // "payload" field and convert to application/json so Dokploy can parse it.
    let body_bytes = if content_type_is_form_urlencoded(&headers) {
        match extract_json_from_form(&body_bytes) {
            Some(json_bytes) => {
                eprintln!("POST {} -> converting form-urlencoded payload to JSON", path);
                builder = builder.header(hyper::header::CONTENT_TYPE, "application/json");
                json_bytes
            }
            None => body_bytes,
        }
    } else {
        body_bytes
    };

    let upstream_req = builder.body(Full::new(body_bytes))?;

    // Send to Dokploy
    let client = Client::builder(TokioExecutor::new()).build_http();
    let upstream_resp = match client.request(upstream_req).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("POST {} -> 502 upstream error: {}", path, e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!("Bad Gateway: {}\n", e))))
                .unwrap());
        }
    };

    // Forward the upstream response back
    let status = upstream_resp.status();
    let resp_body = upstream_resp.into_body().collect().await?.to_bytes();

    eprintln!("POST {} -> upstream responded {}", path, status);

    Ok(Response::builder()
        .status(status)
        .body(Full::new(resp_body))
        .unwrap())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let base_url = env::var("DOKPLOY_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string();

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("webhook-forwarder listening on {}", addr);
    eprintln!("forwarding to {}", base_url);

    loop {
        let (stream, _) = listener.accept().await?;
        let base_url = base_url.clone();

        tokio::task::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(stream);
            let service = service_fn(move |req| {
                let base_url = base_url.clone();
                handle(base_url, req)
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("connection error: {}", e);
            }
        });
    }
}
