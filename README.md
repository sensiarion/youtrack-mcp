# youtrack-mcp

Lean [Model Context Protocol](https://modelcontextprotocol.io) server for [YouTrack](https://www.jetbrains.com/youtrack/), written in Rust. Single static binary, no runtime.

## Why

A drop-in replacement for heavier YouTrack MCPs:

- **~10× smaller tool schema** — 16 consolidated tools (~2.9k tokens in `tools/list`) instead of 50+ (~23k tokens). Less context burned on every request.
- **Correct subtask hierarchy** — parent set reliably via the command API; the real parent issue is surfaced (not an opaque link id).
- **Complete write surface** — tags, agile board/sprint, work-item type for time tracking, attachments, article + comment create/edit, issue delete.
- **Lean responses** — only the fields an agent needs; YouTrack `$type` noise stripped.
- **Safe by design** — never creates tags (only applies existing ones); secrets never leak into error messages.

## Tools

`issue_write` `issue_get` `issue_search` `issue_links` `link_write` `comment_write` `comments_list` `article_write` `article_get` `workitem_write` `workitems_list` `workitems_report` `activity` `users` `meta` `attachment`

---

## Install (no Rust, copy-paste)

You do **not** need Rust or any build tools. Three steps: install the binary, get a token, register the MCP. ~3 minutes.

### 1. Install the binary

**macOS / Linux** — paste into a terminal:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sensiarion/youtrack-mcp/releases/latest/download/youtrack-mcp-installer.sh | sh
```

**Windows** — paste into PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/sensiarion/youtrack-mcp/releases/latest/download/youtrack-mcp-installer.ps1 | iex"
```

The installer drops the binary at a fixed location and adds it to your `PATH`:

| OS | Installed binary path |
|---|---|
| macOS / Linux | `~/.cargo/bin/youtrack-mcp` |
| Windows | `%USERPROFILE%\.cargo\bin\youtrack-mcp.exe` |

> **PATH note:** the installer edits your shell profile so `youtrack-mcp` works in *new* terminals, but the change does **not** apply to the terminal you ran it in until you restart it. To avoid PATH issues entirely, the MCP config in step 3 uses the **full path** above — that always works, no terminal restart needed.

Check it installed (open a **new** terminal):

```sh
~/.cargo/bin/youtrack-mcp --help 2>/dev/null; echo "installed at: $(ls ~/.cargo/bin/youtrack-mcp 2>/dev/null || echo MISSING)"
```

### 2. Get a YouTrack token

In YouTrack: avatar → **Profile** → **Account Security** → **New token…** → scope **YouTrack** → copy the `perm-…` string. Treat it like a password.

### 3. Register the MCP

Two env vars are required: `YOUTRACK_URL` (e.g. `https://youtrack.example.com`) and `YOUTRACK_TOKEN` (the `perm-…` token).

**Claude Code** — one command (uses the full path so PATH does not matter):

```sh
claude mcp add youtrack \
  --env YOUTRACK_URL=https://youtrack.example.com \
  --env YOUTRACK_TOKEN=perm-xxxx \
  -- "$HOME/.cargo/bin/youtrack-mcp"
```

Windows (PowerShell):

```powershell
claude mcp add youtrack --env YOUTRACK_URL=https://youtrack.example.com --env YOUTRACK_TOKEN=perm-xxxx -- "$env:USERPROFILE\.cargo\bin\youtrack-mcp.exe"
```

**Claude Code / any client via JSON** — edit `~/.claude.json` (global) or project `.mcp.json`:

```json
{
  "mcpServers": {
    "youtrack": {
      "command": "/Users/<you>/.cargo/bin/youtrack-mcp",
      "args": [],
      "env": {
        "YOUTRACK_URL": "https://youtrack.example.com",
        "YOUTRACK_TOKEN": "perm-xxxx"
      }
    }
  }
}
```

**Cursor** — `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (project):

```json
{
  "mcpServers": {
    "youtrack": {
      "command": "/Users/<you>/.cargo/bin/youtrack-mcp",
      "env": {
        "YOUTRACK_URL": "https://youtrack.example.com",
        "YOUTRACK_TOKEN": "perm-xxxx"
      }
    }
  }
}
```

Replace `/Users/<you>` with your home directory (`echo $HOME`). On Windows use `C:\\Users\\<you>\\.cargo\\bin\\youtrack-mcp.exe` (double backslashes in JSON). Restart the client; the `youtrack` tools appear.

### Update / uninstall

Re-run the step 1 installer to update. To remove: delete the binary (`rm ~/.cargo/bin/youtrack-mcp`) and the `youtrack` entry from your client config.

---

## Environment variables

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

## Conventions

- Issue ids are readable (`ABC-123`); bare numbers expand via `YOUTRACK_DEFAULT_PROJECT`.
- Parent/child uses `issue_write.parentId` (native subtask) — **not** `link_write`.
- Dates are ISO `YYYY-MM-DD`.
- Tags must already exist; an unknown tag is an error (this server never creates tags).
- `issue_write op=delete` permanently deletes the issue.
- Errors are one-line JSON-RPC errors naming the bad/missing field; `YouTrack <status>: <msg>` means the API rejected the call.

---

## Build from source (developers)

Needs the Rust toolchain. The pinned version in `rust-toolchain.toml` installs automatically via `rustup`.

Install straight from git:

```sh
cargo install --git https://github.com/sensiarion/youtrack-mcp --locked
```

(Binary lands in `~/.cargo/bin/youtrack-mcp` — same path as the prebuilt installer, so the step-3 MCP config is unchanged.)

Or clone and build:

```sh
git clone https://github.com/sensiarion/youtrack-mcp
cd youtrack-mcp
cargo build --release
# binary at target/release/youtrack-mcp
```

Releases (prebuilt binaries + installers for macOS Apple Silicon, Linux arm64/x64, Windows x64) are produced by [cargo-dist](https://opensource.axo.dev/cargo-dist/) on every `v*` tag. Intel Macs are not prebuilt — build from source as above (`cargo install --git …`); the binary path is identical.

## License

MIT
