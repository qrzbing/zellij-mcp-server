# zellij mcp server 文档

功能：

```
Zellij MCP Server written by Rust

Usage: zellij-mcp-server <COMMAND>

Commands:
  run   Run a zellij MCP server
  cli   Debug in a CLI interactivate shell
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## run

```
Run a zellij MCP server

Usage: zellij-mcp-server run [OPTIONS]

Options:
  -b, --bind-address <BIND_ADDRESS>  Address to bind to [default: 127.0.0.1:3000]
  -h, --help                         Print help
```

## cli

```
Debug in a CLI interactivate shell

Usage: zellij-mcp-server cli [OPTIONS]

Options:
  -p, --zellij-path <ZELLIJ_PATH>  Path to the zellij binary
  -s, --socket-path <SOCKET_PATH>  Path to the zellij socket (auto-detected if not specified)
  -h, --help                       Print help
```
