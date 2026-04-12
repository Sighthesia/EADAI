use crate::model::McpServerStatus;
use axum::{Router, routing::get};
use eadai::ai_adapter::AiContextAdapter;
use eadai::mcp_server::TelemetryMcpServer;
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

// FIXME: Surface the embedded MCP bind host through desktop config if remote clients become a product requirement.
const MCP_BIND_HOST: &str = "127.0.0.1";
// FIXME: Surface the embedded MCP port through desktop config if operators need multiple local EADAI instances.
const MCP_BIND_PORT: u16 = 8765;
const MCP_PATH: &str = "/mcp";

pub struct EmbeddedMcpServer {
    status: Arc<Mutex<McpServerStatus>>,
    _worker: JoinHandle<()>,
}

impl EmbeddedMcpServer {
    pub fn new(adapter: AiContextAdapter) -> Self {
        let status = Arc::new(Mutex::new(McpServerStatus::starting()));
        let worker = spawn_server(adapter, Arc::clone(&status));
        Self {
            status,
            _worker: worker,
        }
    }

    pub fn status(&self) -> McpServerStatus {
        lock_status(&self.status).clone()
    }
}

fn spawn_server(
    adapter: AiContextAdapter,
    status: Arc<Mutex<McpServerStatus>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                set_error(&status, format!("failed to create MCP runtime: {error}"));
                return;
            }
        };

        runtime.block_on(async move {
            let (listener, bind_warning) = match bind_listener().await {
                Ok(result) => result,
                Err(error) => {
                    set_error(&status, error);
                    return;
                }
            };
            let address = match listener.local_addr() {
                Ok(address) => address,
                Err(error) => {
                    set_error(&status, format!("failed to resolve embedded MCP address: {error}"));
                    return;
                }
            };

            let adapter_factory = adapter.clone();
            let mcp_service: StreamableHttpService<TelemetryMcpServer, LocalSessionManager> =
                StreamableHttpService::new(
                    move || Ok(TelemetryMcpServer::new(adapter_factory.clone())),
                    LocalSessionManager::default().into(),
                    StreamableHttpServerConfig::default(),
                );

            {
                let mut current = lock_status(&status);
                *current = McpServerStatus::running(format!(
                    "http://{}:{}{}",
                    address.ip(),
                    address.port(),
                    MCP_PATH
                ), bind_warning);
            }

            let app = Router::new()
                .route("/health", get(health_check))
                .nest_service(MCP_PATH, mcp_service);

            if let Err(error) = axum::serve(listener, app).await {
                set_error(&status, format!("embedded MCP server stopped: {error}"));
            }
        });
    })
}

async fn bind_listener() -> Result<(tokio::net::TcpListener, Option<String>), String> {
    match tokio::net::TcpListener::bind((MCP_BIND_HOST, MCP_BIND_PORT)).await {
        Ok(listener) => Ok((listener, None)),
        Err(preferred_error) => tokio::net::TcpListener::bind((MCP_BIND_HOST, 0))
            .await
            .map(|listener| {
                (
                    listener,
                    Some(format!(
                        "preferred MCP port {} unavailable: {}. Using a fallback local port.",
                        MCP_BIND_PORT, preferred_error
                    )),
                )
            })
            .map_err(|fallback_error| {
                format!(
                    "failed to bind embedded MCP server on preferred port {} ({}) and fallback port ({}).",
                    MCP_BIND_PORT, preferred_error, fallback_error
                )
            }),
    }
}

async fn health_check() -> &'static str {
    "OK"
}

fn set_error(status: &Arc<Mutex<McpServerStatus>>, error: String) {
    let mut current = lock_status(status);
    *current = McpServerStatus::failed(error);
}

fn lock_status<'a>(status: &'a Arc<Mutex<McpServerStatus>>) -> MutexGuard<'a, McpServerStatus> {
    match status.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
