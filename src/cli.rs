use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ZellijAction {
    List,
}

#[derive(Parser, Debug)]
pub enum McpOptions {
    #[command(name = "run")]
    Run {
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        bind_address: String,
    },

    #[command(name = "cli")]
    Cli {
        #[arg(short, long, default_value = "zellij")]
        zellij_path: String,

        #[arg(short, long, default_value = "/run/user/1000/zellij/0.43.1")]
        socket_path: String,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Coverage Controller")]
pub struct Cli {
    #[command(subcommand)]
    pub command: McpOptions,
}
