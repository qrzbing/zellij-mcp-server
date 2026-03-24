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
mod interactive;
mod manager;
mod mcp;

use cli::ZellijConfig;
use interactive::InteractiveCli;
use mcp::ZellijMcpServer;

pub async fn run_mcp_server(bind_address: String, config: ZellijConfig) -> anyhow::Result<()> {
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
        move || Ok(ZellijMcpServer::new(config.clone())),
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

    let socket_path = cli.command.socket_path();
    let requested_zellij_path = cli.command.raw_zellij_path();
    let zellij_path = match cli.command.checked_zellij_path() {
        Some(path) => path,
        None => {
            if requested_zellij_path.is_absolute() || requested_zellij_path.components().count() > 1
            {
                eprintln!(
                    "Zellij does not exist at {}",
                    requested_zellij_path.display()
                );
            } else {
                eprintln!(
                    "Zellij executable '{}' was not found in PATH",
                    requested_zellij_path.display()
                );
            }
            std::process::exit(1);
        }
    };

    match cli.command {
        cli::McpOptions::Run {
            bind_address,
            config,
        } => {
            println!("Running on {}", bind_address);
            if let Err(e) = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(run_mcp_server(bind_address, config))
            {
                eprintln!("MCP server error: {}", e);
                std::process::exit(1);
            }
        }
        cli::McpOptions::Cli { config: _config } => {
            let mut interactive_cli =
                InteractiveCli::new(&socket_path, &zellij_path).expect("Failed to initialize CLI");
            if let Err(e) = interactive_cli.run() {
                eprintln!("CLI error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
