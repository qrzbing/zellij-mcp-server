# Documentation

本目录用于存放项目文档，分为两类：

- 参考文档（Reference）
- 设计文档（Design / Architecture）

## Reference

- [CLI Commands](./cli-command.md)
  - 自动生成，描述 `zellij-mcp-server cli` 的命令与参数。
- [MCP Tools](./mcp-command.md)
  - 自动生成，描述 MCP 工具接口与参数。

## Design / Architecture

- [Action Execution And UI Attach Strategy](./architecture-action-execution.md)
  - 说明 action 执行路径、`send_action` / `send_action_as_ui_client` 的策略、
    以及“减少 attach/detach 次数”的当前实现。

## Operations

- [Troubleshooting Guide](./operations-troubleshooting.md)
  - 常见问题的症状、根因、处理步骤与诊断清单。

## Conventions

- `cli-command.md` 与 `mcp-command.md` 由生成脚本维护，不应手工改动。
- 设计文档建议包含：背景、目标、决策、流程、代码位置、限制与后续计划。
