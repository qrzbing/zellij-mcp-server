# Zellij MCP Service Written in Rust

This project is for learning vibe coding.

English | [中文](./README.zh_CN.md)

English version is translated by LLM.

---

## Implementation Ideas

There are already some Zellij MCP Servers, for example:

- [GitJuhb/zellij-mcp-server](https://github.com/GitJuhb/zellij-mcp-server)
- [suprposition/zellij-mcp-server](https://pypi.org/project/zellij-mcp-server/)
- [theslyprofessor/zellij-pane-tracker](https://github.com/theslyprofessor/zellij-pane-tracker)

Among them, GitJuhb/zellij-mcp-server is implemented quite comprehensively. But during actual use, I realized I had some needs:

1. No need to split panes; use tabs to preserve workflows. If pane splitting is used, it is common to split a lot of panes inside one tab, and if the screen is too small, it interferes with reading content information;
2. Most MCP implementations are written in Typescript/Python, making them hard to use on embedded devices. A runtime generated from Rust is very small, and Zellij itself is written in Rust, so it is suitable to call Zellij packages directly;
3. I need to support both stdio/http requests at the same time;

After using GitJuhb/zellij-mcp-server for a while, I chose to write my own code. The current functionality is still not perfect. I borrowed some power from outside the world (Vibe Coding) to help me read the source code of [Zellij](https://github.com/zellij-org/zellij) and [rust-sdk](https://github.com/modelcontextprotocol/rust-sdk), so much of the content may not be best practice. Suggestions and feedback are welcome!

## Build and Run

Dynamic linking:

```sh
just build
```

Static linking:

```sh
just build-static
```

Build with Nix (dynamic/static):

```sh
# Dynamic linking (glibc)
nix build .#zellij-mcp-server-dynamic

# Static linking (musl)
nix build .#zellij-mcp-server-static

# Default package (currently dynamic linking)
nix build .#zellij-mcp-server
```

System service invocation (managed through home manager):

```nix
{
  pkgs,
  ...
}:
{
  home.packages = with pkgs; [
    zellij-mcp-server
  ];

  systemd.user.services = {
    zellij = {
      Unit = {
        Description = "Zellij MCP Service (User Level)";
      };

      Service = {
        Type = "simple";
        ExecStart = "${pkgs.zellij-mcp-server}/bin/zellij-mcp-server run";
        Restart = "on-failure";
        RestartSec = 5;
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
```

Or use the [binary](https://github.com/qrzbing/zellij-mcp-server/releases) published in Release.

## Usage

```
$ zellij-mcp-server --help
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

It currently contains two features:

- MCP service: runs on <http://127.0.0.1:3000>. For more detailed usage, see [MCP Usage Documentation](./docs/mcp-command.md)
- CLI service: used to debug various implemented features. For more detailed usage, see [CLI Usage Documentation](./docs/cli-command.md)
- Project documentation entry: see [Documentation](./docs/README.md)

## TODOs

- [ ] More detailed status information
- [ ] Implement more MCP tools [#1](https://github.com/qrzbing/zellij-mcp-server/issues/1)
- [x] Add Nix release
- [x] Add GitHub Action

---

This project implementation depends on the [zellij-utils](https://crates.io/crates/zellij-utils) version, with the plan as follows:

- 0.x.x: compatible with zellij 0.43.x
  - 0.1.x: compatible with zellij 0.43.1
- 1.x.x: compatible with zellij 0.44.x
  - 1.0.x: compatible with zellij 0.44.0
