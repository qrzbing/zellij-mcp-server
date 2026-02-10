use axum::{Json, Router, routing::get};
use clap::Parser;
use rmcp::transport::{
    StreamableHttpServerConfig, StreamableHttpService,
    streamable_http_server::session::local::LocalSessionManager,
};
use serde_json::json;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod server;
use server::ZellijMcpServer;

pub async fn run_mcp_server(bind_address: String) -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,cov_mcp=debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Live Coverage MCP server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {}", bind_address);

    let ct = tokio_util::sync::CancellationToken::new();

    // Create MCP service
    let service = StreamableHttpService::new(
        || Ok(ZellijMcpServer::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            cancellation_token: ct.child_token(),
            stateful_mode: true,
            ..Default::default()
        },
    );

    // Health check endpoint
    let health_check = get(|| async {
        Json(json!({
            "status": "ok",
            "service": "live-cov-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }))
    });

    // Create HTTP router
    let router = Router::new()
        .route("/health", health_check)
        .nest_service("/mcp", service);

    let tcp_listener = tokio::net::TcpListener::bind(&bind_address).await?;

    info!("✓ MCP server listening at http://{}/mcp", bind_address);
    info!("✓ Health check available at http://{}/health", bind_address);
    info!("✓ Press Ctrl+C to stop");

    // Start server with graceful shutdown
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Received shutdown signal, stopping gracefully...");
            ct.cancel();
        })
        .await?;

    info!("Server stopped");

    Ok(())
}

fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::McpOptions::Run { bind } => {
            println!("Running on {}", bind);
            if let Err(e) = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(run_mcp_server(bind))
            {
                eprintln!("MCP server error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
