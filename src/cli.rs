use std::path::PathBuf;

use clap::{Args, Parser};

#[derive(Args, Debug, Clone)]
pub struct ZellijConfig {
    /// Path to the zellij binary
    #[arg(short = 'p', long)]
    pub zellij_path: Option<String>,

    /// Path to the zellij socket (auto-detected if not specified)
    #[arg(short = 's', long)]
    pub socket_path: Option<String>,
}

impl ZellijConfig {
    /// 获取最终的 socket 路径
    pub fn resolve_socket_path(&self) -> PathBuf {
        // 1. 如果用户显式指定了，直接用用户的
        if let Some(ref path) = self.socket_path {
            return PathBuf::from(path);
        }

        // 2. 否则，执行自动探测逻辑
        if let Some(runtime_dir) = dirs::runtime_dir() {
            let base_dir = runtime_dir.join("zellij");

            if base_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&base_dir) {
                    let version_dirs: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();

                    if let Some(first_version) = version_dirs.first() {
                        return first_version.clone();
                    }
                }
                return base_dir;
            }
            base_dir
        } else {
            PathBuf::from("/tmp/zellij")
        }
    }

    /// 获取最终的 zellij 可执行文件路径
    pub fn resolve_zellij_path(&self) -> PathBuf {
        PathBuf::from(
            self.zellij_path
                .clone()
                .unwrap_or_else(|| "zellij".to_string()),
        )
    }
}

#[derive(Parser, Debug)]
pub enum McpOptions {
    /// Run a zellij MCP server
    #[command(name = "run")]
    Run {
        /// Address to bind to
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        bind_address: String,

        #[command(flatten)]
        config: ZellijConfig,
    },

    /// Debug in a CLI interactivate shell
    #[command(name = "cli")]
    Cli {
        #[command(flatten)]
        config: ZellijConfig,
    },
}

impl McpOptions {
    pub fn socket_path(&self) -> PathBuf {
        match self {
            McpOptions::Run { config, .. } => config.resolve_socket_path(),
            McpOptions::Cli { config, .. } => config.resolve_socket_path(),
        }
    }

    pub fn zellij_path(&self) -> PathBuf {
        match self {
            McpOptions::Run { config, .. } => config.resolve_zellij_path(),
            McpOptions::Cli { config, .. } => config.resolve_zellij_path(),
        }
    }
}

/// Zellij MCP Server written by Rust
#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: McpOptions,
}
