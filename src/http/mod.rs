use std::env;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tower_http::services::{ServeDir, ServeFile};

use crate::result::{RipError, RipResult};
use crate::rip_database::{RipDatabase, RipDbEntry};

const DEFAULT_HTTP_LISTEN_ADDR: &str = "127.0.0.1:8080";

pub type HttpRequestSender = mpsc::Sender<HttpRequest>;
pub type HttpRequestReceiver = mpsc::Receiver<HttpRequest>;

pub struct HttpRequest {
    pub kind: HttpRequestKind,
    pub reply_to: oneshot::Sender<HttpResponse>,
}

pub enum HttpRequestKind {
    GetRoutes,
}

pub enum HttpResponse {
    Routes(Vec<HttpRoute>),
}

#[derive(Clone)]
struct HttpState {
    sender: HttpRequestSender,
}

#[derive(Debug, Clone, Serialize)]
pub struct HttpRoute {
    pub destination: String,
    pub prefix: u32,
    pub netmask: String,
    pub next_hop: String,
    pub metric: u32,
    pub interface_index: u32,
    pub route_type: &'static str,
    pub state: &'static str,
    pub changed: bool,
    pub in_kernel: bool,
    pub timeout_seconds: u64,
}

impl HttpRoute {
    pub fn from_database(database: &RipDatabase) -> Vec<Self> {
        let mut routes: Vec<Self> = database
            .ok_routes
            .values()
            .map(|route| Self::from_db_entry(route, "active"))
            .chain(
                database
                    .garbage_routes
                    .values()
                    .map(|route| Self::from_db_entry(route, "garbage")),
            )
            .collect();

        routes.sort_by_key(|route| {
            (
                route.destination.clone(),
                route.prefix,
                route.next_hop.clone(),
                route.interface_index,
            )
        });
        routes
    }

    fn from_db_entry(route: &RipDbEntry, state: &'static str) -> Self {
        let entry = &route.rip_entry;
        let prefix = entry.subnet_mask.count_ones();
        let route_type = if route.is_local { "local" } else { "remote" };

        Self {
            destination: Ipv4Addr::from(entry.ip_address).to_string(),
            prefix,
            netmask: Ipv4Addr::from(entry.subnet_mask).to_string(),
            next_hop: Ipv4Addr::from(entry.next_hop).to_string(),
            metric: entry.metric,
            interface_index: route.if_index,
            route_type,
            state,
            changed: route.changed,
            in_kernel: route.in_routing_table,
            timeout_seconds: route.timeout_cnt,
        }
    }
}

pub fn create_http_channel() -> (HttpRequestSender, HttpRequestReceiver) {
    mpsc::channel(32)
}

pub async fn spawn_http_server(sender: HttpRequestSender) -> RipResult<()> {
    let listen_addr =
        env::var("RIP_HTTP_LISTEN_ADDR").unwrap_or(DEFAULT_HTTP_LISTEN_ADDR.to_string());
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .map_err(|err| RipError::IoError(err.to_string()))?;
    let static_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/http/static");
    let static_files =
        ServeDir::new(&static_path).fallback(ServeFile::new(static_path.join("index.html")));
    let app = Router::new()
        .route("/api/v1/routes", get(get_routes))
        .fallback_service(static_files)
        .with_state(HttpState { sender });

    log::info!("HTTP server listening on {}", listen_addr);
    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            log::warn!("HTTP server stopped: {}", err);
        }
    });

    Ok(())
}

async fn get_routes(State(state): State<HttpState>) -> impl IntoResponse {
    log::info!("handling HTTP GET /api/v1/routes");
    match send_http_request(&state.sender, HttpRequestKind::GetRoutes).await {
        Ok(HttpResponse::Routes(routes)) => Json(routes).into_response(),
        Err(response) => response,
    }
}

async fn send_http_request(
    sender: &HttpRequestSender,
    kind: HttpRequestKind,
) -> Result<HttpResponse, axum::response::Response> {
    let (reply_to, reply_rx) = oneshot::channel();
    let request = HttpRequest { kind, reply_to };

    if sender.send(request).await.is_err() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "RIP daemon is not accepting admin requests",
        )
            .into_response());
    }

    match reply_rx.await {
        Ok(response) => Ok(response),
        Err(_) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "RIP daemon did not answer admin request",
        )
            .into_response()),
    }
}

#[cfg(test)]
#[path = "../tests/http_tests.rs"]
mod tests;
