# Troubleshooting Guide

- Status: Draft
- Last Updated: 2026-03-06
- Scope: CLI (`zellij-mcp-server cli`) + MCP tools

## 1. How To Use This Guide

按以下顺序排查：

1. 先确认命令是否使用正确（session 命令 vs tab 命令）。
2. 再确认运行的是最新构建（尤其是 `target/release` 二进制）。
3. 最后根据具体报错进入对应章节。

## 2. Quick Checks

### 2.1 Verify Session Exists

```bash
zellij list-sessions
```

或在 CLI 内：

```text
>>> ls s
```

### 2.2 Verify Correct Command Type

- 会话操作：`attach <SESSION_NAME>` / `detach`
- tab 操作：`switch <TAB_NAME>` / `ls t` / `new`

### 2.3 Verify Binary Version Is Rebuilt

当源码改动后，请重新构建：

```bash
cargo build --release
./target/release/zellij-mcp-server cli
```

## 3. Common Issues

### 3.1 `attach "Tab #1"` 报错

#### Symptom

```text
Error: Failed to initialize tab information for session 'Tab #1'
```

#### Root Cause

`attach` 只能接收 session 名；`"Tab #1"` 是 tab 名，不是 session 名。

#### Resolution

先 attach session，再 switch tab：

```text
>>> attach <SESSION_NAME>
>>> switch "Tab #1"
```

### 3.2 `dump screen` 报 `Failed to read temp file`

#### Symptom

```text
Failed to read temp file: /tmp/zellij-dump-xxxx.txt
```

#### Typical Cause

headless 场景中，`DumpScreen` 未成功落盘（例如动作执行异常或会话状态异常）。

#### Resolution

1. 先执行 `ls s` 和 `ls t` 确认会话/标签可读。
2. 再执行一次 `dump screen` 验证是否偶发。
3. 若持续复现，先在一个真实终端 `zellij attach <SESSION_NAME>`，再重试。
4. 仍失败时记录日志并提交 issue（附 session 状态和复现步骤）。

### 3.3 `ls t` 输出不稳定或为空

#### Symptom

- 同一个会话多次 `ls t` 结果不一致。

#### Typical Cause

通常是运行了旧二进制（未包含新的 action-reply / attach-barrier 修复）。

#### Resolution

1. 重新 `cargo build --release`
2. 用 `./target/release/zellij-mcp-server cli` 重跑
3. 连续执行 `ls t` 验证稳定性

### 3.4 `switch` 或 `dump screen` 在 headless 下变慢

#### Symptom

- 无 UI 客户端在线时，`switch` / `dump screen` 比有 UI 客户端时更慢。

#### Root Cause

当前策略是“按需 attach”：headless 需要临时 `AttachClient` 后再执行动作。

#### Resolution

- 这是当前预期行为，功能正确优先于延迟。
- 如需进一步优化，按 roadmap 实现“短时复用 UI 连接”。

## 4. Diagnostics Checklist

提交问题前建议附带：

- 运行命令和完整输出（可脱敏）
- 是否 headless
- `ls s` / `ls t` 输出
- 是否重新构建过 release 二进制
- zellij 版本与本项目 commit SHA

## 5. Escalation Path

若问题无法按本文解决：

1. 先在 `docs/architecture-action-execution.md` 对照当前设计确认预期行为。
2. 再在 issue 中附上最小复现步骤和诊断信息。
