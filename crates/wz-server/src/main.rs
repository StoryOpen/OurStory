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
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use wz::source::{NativeWzSource, WzSource};

static SEARCH_INDEX: OnceLock<SearchIndex> = OnceLock::new();
static INDEX_PATH: OnceLock<PathBuf> = OnceLock::new();
static STATIC_DIR: OnceLock<PathBuf> = OnceLock::new();

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

    /// Optional directory to serve static files from (e.g. wasm client build output)
    #[arg(long)]
    serve_dir: Option<PathBuf>,
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

    if let Some(dir) = &args.serve_dir {
        if dir.is_dir() {
            STATIC_DIR.set(dir.clone()).ok();
            info!("Serving static files from {}", dir.display());
        } else {
            warn!("--serve-dir {} is not a directory, ignoring", dir.display());
        }
    }

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
            // If --serve-dir is set, serve the wasm client's index.html instead
            if let Some(serve_dir) = STATIC_DIR.get() {
                serve_static_file(serve_dir, "/index.html").await
            } else {
                response(StatusCode::OK, "text/html", index_html())
            }
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
            // /wz/data/character/{skin_suffix}/{hair_id}/{face_id}/{action}/{expression}
            let rest = &path["/wz/data/character/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 5 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected skin/hair/face/action/expression".to_vec()));
            }
            let skin: u32 = parts[0].parse().unwrap_or(0);
            let hair: u32 = parts[1].parse().unwrap_or(0);
            let face: u32 = parts[2].parse().unwrap_or(0);
            let action = parts[3];
            let expression = parts[4];
            let wz_data = wz::WzData::global();
            let body = match wz_data.load_character_body(skin, action) {
                Ok(b) => b,
                Err(err) => return Ok(response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes())),
            };
            let hair_body = match wz_data.load_hair_body(hair, action) {
                Ok(h) => h,
                Err(err) => return Ok(response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes())),
            };
            let face_expr = match wz_data.load_face_expression(face, expression) {
                Ok(f) => f,
                Err(err) => return Ok(response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes())),
            };
            #[derive(serde::Serialize)]
            struct CharacterResponse {
                body: std::sync::Arc<wz::CharacterBody>,
                hair: std::sync::Arc<wz::HairBody>,
                face_expression: std::sync::Arc<wz::FaceExpression>,
            }
            let data = CharacterResponse { body, hair: hair_body, face_expression: face_expr };
            match serde_json::to_vec(&data) {
                Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
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

        (&Method::GET, path) if path == "/wz/data/physics" => {
            match wz::WzData::global().load_physics() {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/skill-database" => {
            match wz::WzData::global().load_skill_database() {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/job-catalog" => {
            let wz_data = wz::WzData::global();
            let class_names = match wz_data.list_children("Skill") {
                Ok(names) => names,
                Err(err) => return Ok(response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes())),
            };
            let mut entries: Vec<(u32, String)> = Vec::new();
            for class_name in class_names {
                let Some(job_key) = class_name.strip_suffix(".img") else { continue };
                let Ok(job_id) = job_key.parse::<u32>() else { continue };
                let label: Option<String> = wz_data.read_string(&format!("String/Skill.img/{job_key}/bookName"));
                if let Some(label) = label {
                    let label = label.trim().to_string();
                    if !label.is_empty() {
                        entries.push((job_id, label));
                    }
                }
            }
            entries.sort_by_key(|(id, _)| *id);
            match serde_json::to_vec(&entries) {
                Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/data/action-lists" => {
            let wz_data = wz::WzData::global();
            let basic: Vec<String> = wz_data
                .list_children("Character/00002001.img")
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a != "info")
                .collect();
            let basic_set: std::collections::HashSet<&str> =
                basic.iter().map(|s| s.as_str()).collect();
            let all = wz_data
                .list_children("Character/00002000.img")
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a != "info");
            let composite: Vec<String> = all.filter(|a| !basic_set.contains(a.as_str())).collect();
            #[derive(serde::Serialize)]
            struct ActionListsResponse {
                basic: Vec<String>,
                composite: Vec<String>,
            }
            let data = ActionListsResponse { basic, composite };
            match serde_json::to_vec(&data) {
                Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/character-body/") => {
            let rest = &path["/wz/data/character-body/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected skin/action".to_vec()));
            }
            let skin: u32 = match parts[0].parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid skin".to_vec())),
            };
            let action = parts[1];
            match wz::WzData::global().load_character_body(skin, action) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/hair-body/") => {
            let rest = &path["/wz/data/hair-body/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected hair_id/action".to_vec()));
            }
            let hair_id: u32 = match parts[0].parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid hair_id".to_vec())),
            };
            let action = parts[1];
            match wz::WzData::global().load_hair_body(hair_id, action) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/equip-action/") => {
            let rest = &path["/wz/data/equip-action/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected item_id/action".to_vec()));
            }
            let item_id: i32 = match parts[0].parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid item_id".to_vec())),
            };
            let action = parts[1];
            match wz::WzData::global().load_equip_action(item_id, action) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/data/face-expression/") => {
            let rest = &path["/wz/data/face-expression/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected face_id/expression".to_vec()));
            }
            let face_id: u32 = match parts[0].parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid face_id".to_vec())),
            };
            let expression = parts[1];
            match wz::WzData::global().load_face_expression(face_id, expression) {
                Ok(data) => match serde_json::to_vec(&*data) {
                    Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        // ═══════════════════════════════════════════════════════════
        //  Binary (bincode) API — used by wasm game client
        //  These mirror the JSON endpoints above but return bincode.
        //  Old JSON endpoints kept for WZ Explorer web UI.
        // ═══════════════════════════════════════════════════════════
        (&Method::GET, path) if path.starts_with("/wz/bdata/map/") => {
            let id_str = &path["/wz/bdata/map/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid map id".to_vec())),
            };
            match wz::WzData::global().load_map(id) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/mob/") => {
            let id_str = &path["/wz/bdata/mob/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid mob id".to_vec())),
            };
            match wz::WzData::global().load_mob(id) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/npc/") => {
            let id_str = &path["/wz/bdata/npc/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid npc id".to_vec())),
            };
            match wz::WzData::global().load_npc(id) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/equip/") => {
            let id_str = &path["/wz/bdata/equip/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid equip id".to_vec())),
            };
            match wz::WzData::global().load_equip(id) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/character-body/") => {
            let rest = &path["/wz/bdata/character-body/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected skin/action".to_vec()));
            }
            let skin: u32 = match parts[0].parse() { Ok(v) => v, Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid skin".to_vec())) };
            let action = parts[1];
            match wz::WzData::global().load_character_body(skin, action) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/hair-body/") => {
            let rest = &path["/wz/bdata/hair-body/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected hair_id/action".to_vec()));
            }
            let hair_id: u32 = match parts[0].parse() { Ok(v) => v, Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid hair_id".to_vec())) };
            let action = parts[1];
            match wz::WzData::global().load_hair_body(hair_id, action) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/equip-action/") => {
            let rest = &path["/wz/bdata/equip-action/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected item_id/action".to_vec()));
            }
            let item_id: i32 = match parts[0].parse() { Ok(v) => v, Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid item_id".to_vec())) };
            let action = parts[1];
            match wz::WzData::global().load_equip_action(item_id, action) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bdata/face-expression/") => {
            let rest = &path["/wz/bdata/face-expression/".len()..];
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"expected face_id/expression".to_vec()));
            }
            let face_id: u32 = match parts[0].parse() { Ok(v) => v, Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid face_id".to_vec())) };
            let expression = parts[1];
            match wz::WzData::global().load_face_expression(face_id, expression) {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/bdata/physics" => {
            match wz::WzData::global().load_physics() {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/bdata/zmap" => {
            match wz::WzData::global().load_zmap() {
                Ok(data) => bincode_response(&data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/bdata/skill-database" => {
            match wz::WzData::global().load_skill_database() {
                Ok(data) => bincode_response(&*data),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/bdata/portal-frames" => {
            match wz::WzData::global().load_portal_frames() {
                Ok(frames) => match bincode::serde::encode_to_vec(&frames, bincode::config::standard()) {
                    Ok(bytes) => response(StatusCode::OK, "application/octet-stream", bytes),
                    Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
                },
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path == "/wz/bdata/job-catalog" => {
            let wz_data = wz::WzData::global();
            let class_names = match wz_data.list_children("Skill") {
                Ok(names) => names,
                Err(err) => return Ok(response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes())),
            };
            let mut entries: Vec<(u32, String)> = Vec::new();
            for class_name in class_names {
                let Some(job_key) = class_name.strip_suffix(".img") else { continue };
                let Ok(job_id) = job_key.parse::<u32>() else { continue };
                let label: Option<String> = wz_data.read_string(&format!("String/Skill.img/{job_key}/bookName"));
                if let Some(label) = label {
                    let label = label.trim().to_string();
                    if !label.is_empty() {
                        entries.push((job_id, label));
                    }
                }
            }
            entries.sort_by_key(|(id, _)| *id);
            bincode_response(&entries)
        }
        (&Method::GET, path) if path == "/wz/bdata/action-lists" => {
            let wz_data = wz::WzData::global();
            let basic: Vec<String> = wz_data
                .list_children("Character/00002001.img")
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a != "info")
                .collect();
            let basic_set: std::collections::HashSet<&str> =
                basic.iter().map(|s| s.as_str()).collect();
            let all = wz_data
                .list_children("Character/00002000.img")
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a != "info");
            let composite: Vec<String> = all.filter(|a| !basic_set.contains(a.as_str())).collect();
            #[derive(serde::Serialize)]
            struct ActionListsResponse {
                basic: Vec<String>,
                composite: Vec<String>,
            }
            bincode_response(&ActionListsResponse { basic, composite })
        }

        (&Method::GET, path) if path.starts_with("/wz/bdata/origin/") => {
            let wz_path = decode_path(&path["/wz/bdata/origin/".len()..]);
            match wz::WzData::global().load_origin(&wz_path) {
                Ok(v) => bincode_response(&(v.0, v.1)),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        // ═══════════════════════════════════════════════════════
        //  Bundle endpoints — one GET, returns data + all images
        //  Cacheable by the browser (unlike POST batch).
        // ═══════════════════════════════════════════════════════
        (&Method::GET, path) if path.starts_with("/wz/bundle/map/") => {
            let id_str = &path["/wz/bundle/map/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid map id".to_vec())),
            };
            match make_map_bundle(id).await {
                Ok(bundle) => bincode_response(&bundle),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bundle/mob/") => {
            let id_str = &path["/wz/bundle/mob/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid mob id".to_vec())),
            };
            match make_mob_bundle(id).await {
                Ok(bundle) => bincode_response(&bundle),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bundle/npc/") => {
            let id_str = &path["/wz/bundle/npc/".len()..];
            let id: i32 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"invalid npc id".to_vec())),
            };
            match make_npc_bundle(id).await {
                Ok(bundle) => bincode_response(&bundle),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        (&Method::GET, path) if path.starts_with("/wz/bundle/paths/") => {
            let raw = &path["/wz/bundle/paths/".len()..];
            // Paths are comma-separated, URL-encoded
            let decoded = urlencoding::decode(raw).unwrap_or_default().replace('+', " ");
            let paths: Vec<&str> = decoded.split(',').filter(|s| !s.is_empty()).collect();
            match make_image_bundle(&paths).await {
                Ok(bundle) => bincode_response(&bundle),
                Err(err) => response(StatusCode::NOT_FOUND, "text/plain", err.to_string().into_bytes()),
            }
        }
        // ═══════════════════════════════════════════════════════
        //  Bulk image fetch — POST path→png HashMap
        //  Accepts JSON array of WZ image paths, returns JSON
        //  object mapping each path to its PNG bytes (base64).
        // ═══════════════════════════════════════════════════════
        (&Method::POST, path) if path == "/wz/images" => {
            use http_body_util::BodyExt;
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes().to_vec(),
                Err(_) => return Ok(response(StatusCode::BAD_REQUEST, "text/plain", b"failed to read body".to_vec())),
            };
            let paths: Vec<String> = match serde_json::from_slice(&body) {
                Ok(p) => p,
                Err(e) => {
                    info!("/wz/images: body ({} bytes): {}", body.len(), String::from_utf8_lossy(&body).chars().take(200).collect::<String>());
                    return Ok(response(StatusCode::BAD_REQUEST, "text/plain", format!("expected JSON array: {}", e).into_bytes()));
                }
            };
            info!("/wz/images: received {} paths", paths.len());
            let images = load_images_png(&paths).await;
            match serde_json::to_vec(&images) {
                Ok(bytes) => response(StatusCode::OK, "application/json", bytes),
                Err(err) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", err.to_string().into_bytes()),
            }
        }

        _ => {
            // Try serving static files from --serve-dir
            if let Some(serve_dir) = STATIC_DIR.get() {
                serve_static_file(serve_dir, req.uri().path()).await
            } else {
                response(StatusCode::NOT_FOUND, "text/plain", "not found")
            }
        }
    };

    Ok(response)
}

fn bincode_response<T: serde::Serialize>(data: &T) -> Response<Full<Bytes>> {
    match bincode::serde::encode_to_vec(data, bincode::config::standard()) {
        Ok(bytes) => response(StatusCode::OK, "application/octet-stream", bytes),
        Err(err) => response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "text/plain",
            err.to_string().into_bytes(),
        ),
    }
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
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    parts.headers.insert(
        hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
    Response::from_parts(parts, body)
}

async fn serve_static_file(serve_dir: &Path, request_path: &str) -> Response<Full<Bytes>> {
    // Normalize: strip leading slash, prevent directory traversal
    let clean = request_path
        .trim_start_matches('/')
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "..")
        .collect::<Vec<_>>()
        .join("/");

    let file_path = if clean.is_empty() {
        serve_dir.join("index.html")
    } else {
        serve_dir.join(&clean)
    };

    // Security: ensure the resolved path is within serve_dir
    if !file_path.starts_with(serve_dir) {
        return response(StatusCode::FORBIDDEN, "text/plain", "forbidden");
    }

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let mime = mime_for_path(&file_path);
            response(StatusCode::OK, mime, bytes)
        }
        Err(_) => response(StatusCode::NOT_FOUND, "text/plain", "not found"),
    }
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

// ═══════════════════════════════════════════════════════════
//  Bundle helpers — walk data structs, collect image paths,
//  encode as PNG, return HashMap<path, bytes>.
// ═══════════════════════════════════════════════════════════

fn collect_map_image_paths(map: &wz::MapData) -> Vec<String> {
    let mut paths = Vec::new();
    for bg in &map.backgrounds {
        paths.push(bg.image_path.clone());
        for anim in &bg.animation_frames {
            paths.push(anim.image_path.clone());
        }
    }
    for layer in &map.layers {
        for tile in &layer.tiles {
            paths.push(tile.image_path.clone());
            for anim in &tile.animation_frames {
                paths.push(anim.image_path.clone());
            }
        }
        for obj in &layer.objs {
            paths.push(obj.image_path.clone());
            for anim in &obj.animation_frames {
                paths.push(anim.image_path.clone());
            }
        }
    }
    if let Some(ref mm) = map.minimap {
        paths.push(mm.image_path.clone());
    }
    deduplicate_paths(&mut paths);
    paths
}

fn collect_mob_image_paths(mob: &wz::MobData) -> Vec<String> {
    let mut paths = Vec::new();
    for action in mob.actions.values() {
        for frame in &action.frames {
            for part in &frame.parts {
                paths.push(part.image_path.clone());
            }
        }
    }
    deduplicate_paths(&mut paths);
    paths
}

fn collect_npc_image_paths(npc: &wz::NpcData) -> Vec<String> {
    let mut paths = Vec::new();
    for action in npc.actions.values() {
        for frame in &action.frames {
            paths.push(frame.image_path.clone());
        }
    }
    deduplicate_paths(&mut paths);
    paths
}

fn deduplicate_paths(paths: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));
}

/// Load multiple images from WZ and encode as PNG bytes.
fn load_origin(path: &str) -> Option<(f32, f32)> {
    let node = wz::get_cached_base().at_path(path).ok()?;
    let origin_node = node.try_get("origin")?;
    let v = origin_node.read_origin(&node).ok()?;
    Some((v.0, v.1))
}

async fn load_images_png(paths: &[String]) -> std::collections::HashMap<String, Vec<u8>> {
    let mut images = std::collections::HashMap::new();
    for path in paths {
        match NativeWzSource.image_png(path).await {
            Ok(png_bytes) => {
                images.insert(path.clone(), png_bytes);
            }
            Err(e) => {
                warn!("bundle: failed to load '{}': {}", path, e);
            }
        }
    }
    images
}

async fn make_map_bundle(id: i32) -> Result<wz::MapBundle, Box<dyn std::error::Error>> {
    let data = (*wz::WzData::global().load_map(id)?).clone();
    let paths = collect_map_image_paths(&data);
    let images = load_images_png(&paths).await;
    Ok(wz::MapBundle { data, images })
}

async fn make_mob_bundle(id: i32) -> Result<wz::MobBundle, Box<dyn std::error::Error>> {
    let data = (*wz::WzData::global().load_mob(id)?).clone();
    let paths = collect_mob_image_paths(&data);
    let images = load_images_png(&paths).await;
    Ok(wz::MobBundle { data, images })
}

async fn make_npc_bundle(id: i32) -> Result<wz::NpcBundle, Box<dyn std::error::Error>> {
    let data = (*wz::WzData::global().load_npc(id)?).clone();
    let paths = collect_npc_image_paths(&data);
    let images = load_images_png(&paths).await;
    Ok(wz::NpcBundle { data, images })
}

async fn make_image_bundle(paths: &[&str]) -> Result<wz::ImageBundle, Box<dyn std::error::Error>> {
    let owned: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
    let images = load_images_png(&owned).await;
    let mut origins = std::collections::HashMap::new();
    for path in &owned {
        if let Some(o) = load_origin(path) {
            origins.insert(path.clone(), o);
        }
    }
    Ok(wz::ImageBundle { images, origins })
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
