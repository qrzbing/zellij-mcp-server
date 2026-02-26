# Zellij MCP Server

Real-time Zellij terminal multiplexer control via MCP protocol.

## Typical Workflows

### Single Session (Simplified)

```
1. attach_session(session_name="dev")  // Auto-sets as current
2. new_tab(name="editor")
3. write(text="vim main.rs")
4. switch_tab(tab_name="Tab #1")
5. write(text="cargo build")
6. dump_screen(path="build-output.txt")
7. detach_session()
```

### Multiple Sessions

```
1. attach_session(session_name="dev")
2. attach_session(session_name="test")
3. select_session(session_name="dev")    // Switch to dev
4. write(text="npm run dev")
5. write(text="echo hello", session_name="test")  // Explicit override
6. list_sessions()  // See all sessions
```

### Interactive Development

```
1. attach_session(session_name="myproject")
2. list_tabs()
3. switch_tab(tab_name="editor")
4. write(text="# Starting development")
5. send_key(key="ctrl+l")  // Clear screen
6. write_multiple(commands=["git status", "git diff"])
7. dump_screen()  // View output
```

## Notes

- All tab/readwrite operations default to the current session if not specified
- Use `session_name` parameter to explicitly target a different session
- First attached session automatically becomes the current session
- Detaching the current session clears the current session state
