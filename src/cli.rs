use std::{
    env,
    path::{Path, PathBuf},
};

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
        if let Some(ref path) = self.socket_path {
            return PathBuf::from(path);
        }
        zellij_utils::consts::ZELLIJ_SOCK_DIR.clone()
    }

    pub fn raw_zellij_path(&self) -> PathBuf {
        PathBuf::from(
            self.zellij_path
                .clone()
                .unwrap_or_else(|| "zellij".to_string()),
        )
    }

    /// 查找 zellij 可执行文件，支持绝对路径、相对路径和 PATH 搜索
    pub fn checked_zellij_path(&self) -> Option<PathBuf> {
        resolve_command_path(&self.raw_zellij_path(), env::var_os("PATH"))
    }

    /// 获取最终的 zellij 可执行文件路径
    pub fn resolve_zellij_path(&self) -> PathBuf {
        self.checked_zellij_path()
            .unwrap_or_else(|| self.raw_zellij_path())
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

    pub fn raw_zellij_path(&self) -> PathBuf {
        match self {
            McpOptions::Run { config, .. } => config.raw_zellij_path(),
            McpOptions::Cli { config, .. } => config.raw_zellij_path(),
        }
    }

    pub fn checked_zellij_path(&self) -> Option<PathBuf> {
        match self {
            McpOptions::Run { config, .. } => config.checked_zellij_path(),
            McpOptions::Cli { config, .. } => config.checked_zellij_path(),
        }
    }
}

fn resolve_command_path(command: &Path, path_env: Option<std::ffi::OsString>) -> Option<PathBuf> {
    if command.is_absolute() || command.components().count() > 1 {
        return is_executable_file(command).then(|| command.to_path_buf());
    }

    let command_name = command.file_name()?;
    for dir in env::split_paths(&path_env?) {
        let candidate = dir.join(command_name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

/// Zellij MCP Server written by Rust
#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: McpOptions,
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::resolve_command_path;

    fn unique_test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zellij-mcp-{name}-{suffix}"))
    }

    fn create_executable(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"#!/bin/sh\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn resolve_command_path_searches_path_for_bare_command_name() {
        let dir = unique_test_dir("path-search");
        let command = dir.join("zellij");
        create_executable(&command);

        let resolved =
            resolve_command_path(Path::new("zellij"), Some(dir.as_os_str().to_os_string()));

        assert_eq!(resolved, Some(command));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolve_command_path_uses_explicit_absolute_path() {
        let dir = unique_test_dir("absolute-path");
        let command = dir.join("custom-zellij");
        create_executable(&command);

        let resolved = resolve_command_path(&command, None);

        assert_eq!(resolved, Some(command.clone()));

        fs::remove_dir_all(dir).unwrap();
    }
}
