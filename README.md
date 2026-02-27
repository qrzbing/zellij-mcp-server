# Rust 编写的 Zellij MCP 服务

本项目用于学习 vibe coding。

---

## 实现想法

目前已经有了一些 Zellij MCP Server，例如

- [GitJuhb/zellij-mcp-server](https://github.com/GitJuhb/zellij-mcp-server)
- [suprposition/zellij-mcp-server](https://pypi.org/project/zellij-mcp-server/)
- [theslyprofessor/zellij-pane-tracker](https://github.com/theslyprofessor/zellij-pane-tracker)

其中 GitJuhb/zellij-mcp-server 实现得相当全面。但在具体使用时，我意识到我有一些需求：

1. 不必拆分面板，用 tab 保存工作流。如果使用 panel 拆分的话经常会在一个 tab 中拆出一堆 panel，如果屏幕太小的话会干扰到读取的内容信息；
2. 大部分 MCP 都是 Typescript/Python 编写的，很难在嵌入式设备上使用。用 Rust 编写生成的运行时很小，而且 Zellij 本身是用 Rust 编写的，适合直接调 Zellij 的包；
3. 我需要同时支持 stdio/http 请求；

在使用了一阵子 GitJuhb/zellij-mcp-server 后，我选择了自己编写代码。目前的功能仍不算完善，我借助了一些世界之外（Vibe Coding）的力量来为我阅读 [Zellij](https://github.com/zellij-org/zellij) 和 [rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) 的源码，因此很多内容可能不是最佳实践。欢迎提出建议和意见！

## 编译运行

动态链接

```sh
just build
```

静态链接

```sh
just build-static
```

或使用 Release 中发布的二进制（Coming Soon!）

## 使用方法

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

目前包含两个功能：

- MCP 服务：运行在 <http://127.0.0.1:3000>，更详细的用法见 [MCP 使用文档](./docs/mcp-command.md)
- CLI 服务：用于调试实现的各类功能，更详细的用法见 [CLI 使用文档](./docs/cli-command.md)

## TODOs

- [ ] 实现更多 MCP 工具 [#1](https://github.com/qrzbing/zellij-mcp-server/issues/1)
- [x] 添加 Nix 发布
- [x] 添加 GitHub Action

---

本项目实现依赖 [zellij-utils](https://crates.io/crates/zellij-utils) 版本，计划为：

- 0.x.x：与 zellij 0.43.x 版本兼容
  - 0.1.x：与 zellij 0.43.1 版本兼容
  - 0.2.x：与 zellij 0.43.2 版本兼容
- 1.x.x：与 zellij 0.44.x 版本兼容
