# youtrack-mcp

Lean [Model Context Protocol](https://modelcontextprotocol.io) server for [YouTrack](https://www.jetbrains.com/youtrack/), written in Rust. Single static binary, no runtime.

## Why

A drop-in replacement for heavier YouTrack MCPs:

- **~10× smaller tool schema** — 16 consolidated tools (~2.9k tokens in `tools/list`) instead of 50+ (~23k tokens). Less context burned on every request.
- **Correct subtask hierarchy** — parent set reliably via the command API; the real parent issue is surfaced (not an opaque link id).
- **Complete write surface** — tags, agile board/sprint, work-item type for time tracking, attachments, article + comment create/edit, issue delete.
- **Safe by design** — never creates tags (only applies existing ones); secrets never leak into error messages.

## Tools

16 consolidated tools. Each takes an `op`/`kind` discriminator where it covers several operations.

| Tool | What it does |
|---|---|
| `issue_write` | Create, update or delete an issue: summary, description, `parentId` (native subtask), assignee, tags (must exist), state, board/sprint. `op=delete` is irreversible. |
| `issue_get` | Get a single issue by id with full fields. |
| `issue_search` | Search issues by YouTrack query. `fields` short \| full. |
| `issue_links` | List links of an issue (direction, type, related issues). |
| `link_write` | Add or remove an issue link. `role` outward \| inward for directed types. For parent/child use `issue_write.parentId`. |
| `comment_write` | Create or update a comment on an issue or article (entity). `op=update` needs `commentId`. |
| `comments_list` | List comments of an issue or article (entity). |
| `article_write` | Create or update a knowledge-base article. |
| `article_get` | Get an article by id (`op get`) or list articles (`op list`, optional query). |
| `workitem_write` | Create / update / delete a time-tracking work item. `type` = work item type name/id. `idempotent` skips duplicates. |
| `workitems_list` | List / aggregate work items by author and date range. |
| `workitems_report` | Per-day expected-vs-actual worktime report (480m/day, skips weekends/holidays). |
| `activity` | Activity feed. `scope=issue` (needs `issueId`, optional author) \| `user` (needs author, defaults last 30d). Categories default to CustomFieldCategory, CommentsCategory. Dates ISO or unix ms. |
| `users` | Users: `op` list (optional query) \| me \| get (by id). |
| `meta` | Discovery: `kind` projects \| link_types \| work_item_types (optional project). |
| `attachment` | Issue attachments: `op` list \| get \| upload \| download \| delete. Upload via `contentBase64`; download returns base64 unless a path is given. |

---

## Install

No Rust, no build step, no binary to manage. The prebuilt server is fetched and run by [mcp-bin](https://github.com/sensiarion/mcp-bin) via `npx`. Only requirement: **Node ≥ 22**. Works on macOS, Linux and Windows. ~2 minutes.

### 1. Get a YouTrack token

In YouTrack: avatar → **Profile** → **Account Security** → **New token…** → scope **YouTrack** → copy the `perm-…` string. Treat it like a password.

### 2. Register the MCP

Two env vars are required: `YOUTRACK_URL` (e.g. `https://youtrack.example.com`) and `YOUTRACK_TOKEN` (the `perm-…` token).

**Claude Code** — one command:

```sh
claude mcp add youtrack \
  --env YOUTRACK_URL=https://youtrack.example.com \
  --env YOUTRACK_TOKEN=perm-xxxx \
  -- npx -y mcp-bin sensiarion/youtrack-mcp
```

**Any client via JSON** — Claude Code `~/.claude.json` (global) or project `.mcp.json`; Cursor `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (project):

```json
{
  "mcpServers": {
    "youtrack": {
      "command": "npx",
      "args": ["-y", "mcp-bin", "sensiarion/youtrack-mcp"],
      "env": {
        "YOUTRACK_URL": "https://youtrack.example.com",
        "YOUTRACK_TOKEN": "perm-xxxx"
      }
    }
  }
}
```

Pin a version by replacing `sensiarion/youtrack-mcp` with `sensiarion/youtrack-mcp@v0.1.2`. Restart the client; the `youtrack` tools appear.

### Update / uninstall

mcp-bin caches a release and reuses it forever by default. To move to a newer release run `npx mcp-bin expire sensiarion/youtrack-mcp` (or pin a tag, or add `--ttl 7d` before the repo in `args`). To uninstall, remove the `youtrack` entry from your client config.

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

(Binary lands in `~/.cargo/bin/youtrack-mcp`; point your MCP client's `command` at that path instead of `npx mcp-bin`.)

Or clone and build:

```sh
git clone https://github.com/sensiarion/youtrack-mcp
cd youtrack-mcp
cargo build --release
# binary at target/release/youtrack-mcp
```

Prebuilt release binaries (macOS Apple Silicon, Linux arm64/x64, Windows x64) — the assets `mcp-bin` downloads — are produced by [cargo-dist](https://opensource.axo.dev/cargo-dist/) on every `v*` tag. Intel Macs are not prebuilt; build from source as above.

## License

MIT
