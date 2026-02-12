# Zellij MCP Server

Real-time Zellij terminal multiplexer control via MCP protocol.

## Available Tools

### Session Management

1. **attach_session** - Attach to a Zellij session
   - Parameters:
     - session_name (string): Name of the session to attach
   - Example: `attach_session(session_name="dev")`
   - Note: Automatically sets as current session

2. **detach_session** - Detach from a Zellij session
   - Parameters:
     - session_name (string, optional): Session to detach (uses current if not provided)
   - Example: `detach_session(session_name="dev")`

3. **list_sessions** - List all available Zellij sessions
   - No parameters required
   - Shows which sessions are attached and which is current

4. **select_session** - Set the current active session
   - Parameters:
     - session_name (string): Session to set as current
   - Example: `select_session(session_name="dev")`

5. **get_current_session** - Get current active session name
   - No parameters required

6. **rename_session** - Rename a session
   - Parameters:
     - new_name (string): New name for the session
     - session_name (string, optional): Session to rename (uses current if not provided)
   - Example: `rename_session(new_name="production")`

### Tab Management

7. **new_tab** - Create a new tab
   - Parameters:
     - name (string, optional): Name for the new tab
     - session_name (string, optional): Session (uses current if not provided)
   - Example: `new_tab(name="editor")`

8. **close_tab** - Close the current tab
   - Parameters:
     - session_name (string, optional): Session (uses current if not provided)
   - Example: `close_tab()`

9. **list_tabs** - List all tabs in the session
   - Parameters:
     - session_name (string, optional): Session (uses current if not provided)
   - Shows which tab is currently focused

10. **switch_tab** - Switch to a specific tab
    - Parameters:
      - tab_name (string): Name of the tab to switch to
      - session_name (string, optional): Session (uses current if not provided)
    - Example: `switch_tab(tab_name="editor")`

11. **rename_tab** - Rename the current tab
    - Parameters:
      - new_name (string, optional): New name (null to undo rename)
      - session_name (string, optional): Session (uses current if not provided)
    - Examples:
      - `rename_tab(new_name="editor")` - Rename to "editor"
      - `rename_tab(new_name=null)` - Undo rename

12. **show_layout** - Show the current session layout
    - Parameters:
      - session_name (string, optional): Session (uses current if not provided)
    - Returns layout in KDL format

### Read/Write Operations

13. **write** - Write text to the current tab
    - Parameters:
      - text (string): Text to write (supports escape sequences)
      - add_newline (bool, optional): Add newline at end (default: true)
      - session_name (string, optional): Session (uses current if not provided)
    - Escape sequences: `\n`, `\r`, `\t`, `\0`, `\e`, `\\`, `\xHH`
    - Examples:
      - `write(text="echo hello")` - Execute command
      - `write(text="hello", add_newline=false)` - Write without Enter

14. **write_multiple** - Write multiple commands
    - Parameters:
      - commands (array of strings): Commands to write
      - session_name (string, optional): Session (uses current if not provided)
    - Example: `write_multiple(commands=["cd /tmp", "ls -la"])`

15. **send_key** - Send a special key
    - Parameters:
      - key (string): Key to send
      - session_name (string, optional): Session (uses current if not provided)
    - Supported keys:
      - Modifiers: `ctrl+c`, `ctrl+d`, `alt+x`
      - Special: `enter`, `tab`, `backspace`, `esc`, `delete`
      - Navigation: `up`, `down`, `left`, `right`, `home`, `end`
      - Function: `f1` through `f12`
    - Examples:
      - `send_key(key="ctrl+c")` - Send Ctrl+C
      - `send_key(key="enter")` - Send Enter

16. **dump_screen** - Dump screen content
    - Parameters:
      - path (string, optional): File path to save (stdout if not provided)
      - full (bool, optional): Include full scrollback (default: false)
      - session_name (string, optional): Session (uses current if not provided)
    - Examples:
      - `dump_screen(path="output.txt")` - Save to file
      - `dump_screen(full=true)` - Dump with full history

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
