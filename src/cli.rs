use clap::Parser;

#[derive(Parser, Debug)]
pub enum McpOptions {
    #[command(name = "run")]
    Run {
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        bind: String,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Coverage Controller")]
pub struct Cli {
    #[command(subcommand)]
    pub command: McpOptions,
}
