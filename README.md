# youtrack-mcp

Lean [Model Context Protocol](https://modelcontextprotocol.io) server for [YouTrack](https://www.jetbrains.com/youtrack/), written in Rust. Single static binary, no runtime.

## Why

A drop-in replacement for heavier YouTrack MCPs:

- **~10× smaller tool schema** — 16 consolidated tools (~2.9k tokens in `tools/list`) instead of 50+ (~23k tokens). Less context burned on every request.
- **Correct subtask hierarchy** — parent set via the native `parent` field, not an unreliable "Subtask" link.
- **Complete write surface** — tags, agile board/sprint, work-item type for time tracking, attachments, article + comment create/edit, issue delete.
- **Lean responses** — only the fields an agent needs; YouTrack `$type` noise stripped.
- **Safe by design** — never creates tags (only applies existing ones); secrets never leak into error messages.

## Tools

`issue_write` `issue_get` `issue_search` `issue_links` `link_write` `comment_write` `comments_list` `article_write` `article_get` `workitem_write` `workitems_list` `workitems_report` `activity` `users` `meta` `attachment`

## Install

### Prebuilt binary (recommended)

macOS / Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sensiarion/youtrack-mcp/releases/latest/download/youtrack-mcp-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/sensiarion/youtrack-mcp/releases/latest/download/youtrack-mcp-installer.ps1 | iex"
```

Or grab a platform archive from the [latest release](https://github.com/sensiarion/youtrack-mcp/releases/latest).

### From source (cargo)

```sh
cargo install --git https://github.com/sensiarion/youtrack-mcp --locked
```

### Manual build

```sh
git clone https://github.com/sensiarion/youtrack-mcp
cd youtrack-mcp
cargo build --release
# binary at target/release/youtrack-mcp
```

The pinned toolchain (`rust-toolchain.toml`) installs automatically via `rustup`.

## Configure

Two env vars required:

| Var | Required | Description |
|---|---|---|
| `YOUTRACK_URL` | yes | Base URL, e.g. `https://youtrack.example.com` |
| `YOUTRACK_TOKEN` | yes | Permanent token (Bearer) |
| `YOUTRACK_DEFAULT_PROJECT` | no | Bare numeric ids expand to `PROJ-<n>` |
| `YOUTRACK_TIMEZONE` | no | IANA tz for report bucketing (default `Europe/Moscow`) |
| `YOUTRACK_HOLIDAYS` | no | CSV ISO dates, skipped in `workitems_report` |
| `YOUTRACK_PRE_HOLIDAYS` | no | CSV ISO dates, counted at 7/8 norm |
| `YOUTRACK_USER_ALIASES` | no | CSV `alias:login` pairs |
| `YOUTRACK_DOWNLOAD_DIR` | no | Default target dir for attachment download |

### Claude Code

```sh
claude mcp add youtrack \
  --env YOUTRACK_URL=https://youtrack.example.com \
  --env YOUTRACK_TOKEN=perm-xxxx \
  -- youtrack-mcp
```

Or edit `~/.claude.json` / project `.mcp.json`:

```json
{
  "mcpServers": {
    "youtrack": {
      "command": "youtrack-mcp",
      "args": [],
      "env": {
        "YOUTRACK_URL": "https://youtrack.example.com",
        "YOUTRACK_TOKEN": "perm-xxxx"
      }
    }
  }
}
```

### Cursor

`~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (project):

```json
{
  "mcpServers": {
    "youtrack": {
      "command": "youtrack-mcp",
      "env": {
        "YOUTRACK_URL": "https://youtrack.example.com",
        "YOUTRACK_TOKEN": "perm-xxxx"
      }
    }
  }
}
```

Use the absolute path (e.g. `/usr/local/bin/youtrack-mcp` or `~/.cargo/bin/youtrack-mcp`) if the binary is not on the client's `PATH`.

### Any MCP client

Transport is stdio. Run `youtrack-mcp` with the two env vars set; speak MCP over stdin/stdout.

## Conventions

- Issue ids are readable (`ABC-123`); bare numbers expand via `YOUTRACK_DEFAULT_PROJECT`.
- Parent/child uses `issue_write.parentId` (native subtask) — **not** `link_write`.
- Dates are ISO `YYYY-MM-DD`.
- Tags must already exist; an unknown tag is an error (this server never creates tags).
- `issue_write op=delete` permanently deletes the issue.
- Errors are one-line JSON-RPC errors naming the bad/missing field; `YouTrack <status>: <msg>` means the API rejected the call.

## License

MIT
