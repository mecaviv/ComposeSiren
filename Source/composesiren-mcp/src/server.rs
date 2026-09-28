//! HTTP server. Streamable HTTP at `/mcp`, plus a health check at `/` and `/health`.

use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::discovery::{self, Instance, FIRST_PORT, PORT_ATTEMPTS};
use crate::dispatch::Dispatch;
use crate::tools::ComposeSirenServer;

/// A running server. Dropping it unregisters the discovery entry and stops the runtime.
pub struct Running {
    runtime: Option<tokio::runtime::Runtime>,
    cancel: CancellationToken,
    port: u16,
    discovery_path: std::path::PathBuf,
}

impl Running {
    /// Bind `127.0.0.1`, register the discovery file, and serve until drop.
    pub fn start(
        plugin_name: &str,
        plugin_4cc: &str,
        standalone: bool,
        dispatch: Dispatch,
    ) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| err.to_string())?;
        let cancel = CancellationToken::new();
        let dispatch_for_task = dispatch;
        let cancel_for_task = cancel.clone();
        let plugin_name = plugin_name.to_owned();
        let plugin_4cc = plugin_4cc.to_owned();

        let (port, listener) = runtime
            .block_on(bind_first_free())
            .map_err(|err| err.to_string())?;

        let instance = Instance {
            plugin_name: plugin_name.clone(),
            plugin_4cc: plugin_4cc.clone(),
            port,
            pid: discovery::current_pid(),
            session_id: discovery::session_id_from_env(),
            standalone,
        };
        let discovery_path = discovery::default_path();
        discovery::register(&discovery_path, &instance).map_err(|err| err.to_string())?;

        let health = json!({
            "status": "ok",
            "protocol": "mcp",
            "pluginName": plugin_name,
            "plugin4CC": plugin_4cc,
            "port": port,
            "standalone": standalone,
            "mcp": format!("http://127.0.0.1:{port}/mcp"),
        });

        runtime.spawn(async move {
            if let Err(err) = serve(listener, dispatch_for_task, health, cancel_for_task).await {
                eprintln!("composesiren-mcp stopped: {err}");
            }
        });

        Ok(Self {
            runtime: Some(runtime),
            cancel,
            port,
            discovery_path,
        })
    }

    /// Port the server bound.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // Advertise the instance as gone before the runtime winds down, so a
        // client does not keep a port whose listener is already stopping.
        let _ = discovery::unregister(&self.discovery_path, self.port);
        self.cancel.cancel();
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        }
    }
}

async fn bind_first_free() -> std::io::Result<(u16, TcpListener)> {
    let mut last_error = None;
    for offset in 0..PORT_ATTEMPTS {
        let port = FIRST_PORT + offset;
        match TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await {
            Ok(listener) => {
                let actual = listener.local_addr()?.port();
                return Ok((actual, listener));
            }
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "no MCP port was free")
    }))
}

async fn serve(
    listener: TcpListener,
    dispatch: Dispatch,
    health: Value,
    cancel: CancellationToken,
) -> Result<(), String> {
    let service = StreamableHttpService::new(
        move || Ok(ComposeSirenServer::new(dispatch)),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_cancellation_token(cancel.clone()),
    );
    let health_body = health.to_string();
    let app = axum::Router::new()
        .route(
            "/",
            axum::routing::get({
                let body = health_body.clone();
                move || {
                    let body = body.clone();
                    async move { axum::Json(serde_json::from_str::<Value>(&body).unwrap_or(json!({}))) }
                }
            }),
        )
        .route(
            "/health",
            axum::routing::get({
                let body = health_body;
                move || {
                    let body = body.clone();
                    async move { axum::Json(serde_json::from_str::<Value>(&body).unwrap_or(json!({}))) }
                }
            }),
        )
        .nest_service("/mcp", service);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await
        .map_err(|err| err.to_string())
}
