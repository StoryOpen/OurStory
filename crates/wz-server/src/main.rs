mod search;

use clap::Parser;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::header::{CONTENT_TYPE, HeaderValue};
use hyper::server::conn::http1;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use search::SearchIndex;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tracing::{error, info};
use wz::source::{NativeWzSource, WzSource};

static SEARCH_INDEX: OnceLock<SearchIndex> = OnceLock::new();
static INDEX_PATH: OnceLock<PathBuf> = OnceLock::new();

fn index_html() -> &'static str {
    static HTML: OnceLock<String> = OnceLock::new();
    HTML.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("index.html");
        std::fs::read_to_string(&path).unwrap_or_else(|_| {
            include_str!("../index.html").to_string()
        })
    })
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: SocketAddr,

    #[arg(long)]
    build_index: bool,

    #[arg(long, default_value = "./wz/search-index.json")]
    index_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    INDEX_PATH.set(args.index_path.clone()).ok();

    if args.build_index {
        info!("Building search index...");
        wz::resolve_base()?;
        let index = SearchIndex::build();
        index.save(&args.index_path)?;
        info!("Search index saved to {}", args.index_path.display());
        return Ok(());
    }

    let index = SearchIndex::load(&args.index_path)
        .map_err(|e| format!("Failed to load search index from {}: {e}. Run with --build-index first.", args.index_path.display()))?;
    SEARCH_INDEX.set(index).ok();

    let listener = TcpListener::bind(args.bind).await?;
    info!("wz-server listening on http://{}", args.bind);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, hyper::service::service_fn(route))
                .await
            {
                error!(?err, "connection failed");
            }
        });
    }
}

fn get_query_param(query: Option<&str>, name: &str) -> Option<String> {
    query?.split('&').find_map(|part| {
        let mut split = part.splitn(2, '=');
        let key = split.next()?;
        let val = split.next()?;
        if key == name {
            let decoded = urlencoding::decode(val)
                .ok()?
                .replace('+', " ");
            Some(decoded)
        } else {
            None
        }
    })
}

async fn route(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    if req.method() == Method::OPTIONS {
        return Ok(cors_response(Response::new(Full::default())));
    }
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") | (&Method::GET, "/index.html") => {
            response(StatusCode::OK, "text/html", index_html())
        }
        (&Method::GET, path) if path == "/wz/search" => {
            let query = req.uri().query();
            let q = get_query_param(query, "q").unwrap_or_default();
            let category = get_query_param(query, "category");
            if q.trim().is_empty() {
                response(
                    StatusCode::OK,
                    "application/json",
                    serde_json::to_vec(&search::SearchResult {
                        results: vec![],
                        categories: vec!["All".into()],
                    })
                    .unwrap_or_default(),
                )
            } else if let Some(index) = SEARCH_INDEX.get() {
                let result = index.search(&q, category.as_deref());
                match serde_json::to_vec(&result) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "text/plain",
                        err.to_string().into_bytes(),
                    ),
                }
            } else {
                response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "text/plain",
                    b"search index not loaded".to_vec(),
                )
            }
        }
        (&Method::GET, path) if path == "/wz/node/" || path == "/wz/node" => {
            let depth = req
                .uri()
                .query()
                .and_then(|query| query.split('&').find_map(|part| part.strip_prefix("depth=")))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(usize::MAX);

            match wz::get_cached_base().to_payload_depth(depth) {
                Ok(payload) => match serde_json::to_vec(&payload) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/node/") => {
            let wz_path = decode_path(&path["/wz/node/".len()..]);
            let depth = req
                .uri()
                .query()
                .and_then(|query| query.split('&').find_map(|part| part.strip_prefix("depth=")))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(usize::MAX);

            match NativeWzSource.node_payload_depth(&wz_path, depth).await {
                Ok(payload) => match serde_json::to_vec(&payload) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/image/") => {
            let wz_path = decode_path(&path["/wz/image/".len()..]);
            match NativeWzSource.image_png(&wz_path).await {
                Ok(bytes) => response(StatusCode::OK, "image/png", bytes),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/map/") => {
            let id_str = &path["/wz/data/map/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid map id".to_vec())),
            };
            match wz::WzData::global().load_map(id) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/mob/") => {
            let id_str = &path["/wz/data/mob/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid mob id".to_vec())),
            };
            match wz::WzData::global().load_mob(id) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/npc/") => {
            let id_str = &path["/wz/data/npc/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid npc id".to_vec())),
            };
            match wz::WzData::global().load_npc(id) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/character/") => {
            // /wz/data/character/{skin_suffix}/{hair_id}/{face_id}
            let rest = &path["/wz/data/character/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 3 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected skin/hair/face".to_vec()));
            }
            let skin: u32 = parts[0].parse().unwrap_or(0);
            let hair: u32 = parts[1].parse().unwrap_or(0);
            let face: u32 = parts[2].parse().unwrap_or(0);
            match wz::WzData::global().load_character(skin, hair, face) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/equip/") => {
            let id_str = &path["/wz/data/equip/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid equip id".to_vec())),
            };
            match wz::WzData::global().load_equip(id) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/zmap" => {
            match wz::WzData::global().load_zmap() {
                Ok(data) => match serde_json::to_vec(&data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/smap" => {
            match wz::WzData::global().load_smap() {
                Ok(data) => match serde_json::to_vec(&data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/physics" => {
            match wz::WzData::global().load_physics() {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        _ => response(StatusCode::NOT_FOUND, "text/plain", "not found"),
    };

    Ok(response)
}

fn response(status: StatusCode, content_type: &'static str, body: impl Into<Bytes>) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(body.into()));
    *response.status_mut() = status;
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    cors_response(response)
}

fn cors_response(response: Response<Full<Bytes>>) -> Response<Full<Bytes>> {
    let (mut parts, body) = response.into_parts();
    parts.headers.insert(
        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    parts.headers.insert(
        hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, OPTIONS"),
    );
    parts.headers.insert(
        hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
    Response::from_parts(parts, body)
}

fn decode_path(path: &str) -> String {
    let decoded = path.replace("%2F", "/").replace("%2f", "/");
    // Strip leading root-name segment (e.g. "Base/") from paths returned by
    // node.path() that include the root node name. at_path() navigates from
    // the root's children, so "Base/Map/..." should become "Map/...".
    static ROOT_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let root = ROOT_NAME.get_or_init(|| {
        wz::get_cached_base().name()
    });
    if let Some(stripped) = decoded.strip_prefix(&format!("{root}/")) {
        stripped.to_string()
    } else {
        decoded
    }
}
