# observability/

`poc.sh` — tracee-based baseline-vs-evasion harness for ordo, pattern
adopted from `rscaller/scripts/poc.sh` (minus the Loki/Promtail stack — the
tracee container's JSON log is filtered directly, nothing else needed).

Two capture windows on the target, each under a fresh tracee container:

1. **baseline** — `curl http://127.0.0.1:<port>/tool -o /tmp/...` (+ chmod +
   run). Tracee sees: execve of curl, socket connects.
2. **evasion** — `ordo send tool -t ssh:<host> -r /tmp/... [--exec]`.
   Tracee sees: no downloader events (only the downloader query is
   reported — chunk/exec noise is filtered out).

The tool is a harmless script served by a throwaway `python3 -m http.server`
on the target — fully self-contained, no internet needed. Verdict: PASS when
baseline ≥1 downloader event and evasion 0.

## Usage

```bash
make poc TARGET=dev-vm-2     # both scenarios
make poc-exec                # baseline curl+run vs ordo send --exec
make poc-download            # baseline curl vs ordo send (transfer only)
make poc-local               # localhost (needs sudo docker)
make poc ARGS="--query curl,wget,wget2"   # passthrough flags

# or the script directly:
bash observability/poc.sh --target dev-vm-2 --scenario exec
bash observability/poc.sh --help
```

Requires on the target: docker (+sudo), python3. Tracee image
(`aquasec/tracee:latest`) is pulled on first run.

Flags: `--target <ssh-host>` (default local), `--events`, `--query`
(downloader indicators, default `curl,wget`), `--port`, `--ordo <binary>`.

## Sample result (dev-vm-2)

```
baseline (curl):  5 event(s)   # execve curl + security_socket_connect
evasion  (ordo):  0 event(s)   # nothing downloader-shaped
PASS — tracee saw the curl/wget baseline but no downloader activity during the ordo transfer.
```
