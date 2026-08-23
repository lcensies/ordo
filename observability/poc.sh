#!/usr/bin/env bash
# poc.sh — tracee observability PoC for ordo.
#
# Proves: tracee flags a classic downloader (curl/wget) fetching a tool,
# but an ordo transfer (shell one-liners over an existing channel) shows no
# downloader activity — with --exec, only the execve of the tool itself.
#
# Two tracee capture windows on the target:
#   baseline: curl <http-server>/tool.sh -o DEST && chmod +x DEST && DEST
#   evasion:  ordo send tool.sh -t <backend> -r DEST [--exec]
#
# A throwaway python http.server on the target serves the tool, so the PoC
# is fully self-contained/offline. Runs locally (default) or against an SSH
# host (--target). Pattern adopted from rscaller/scripts/poc.sh.
#
# Usage:
#   bash observability/poc.sh [OPTIONS]
#
# Options:
#   --scenario <name>  exec|download  (default: exec)
#                      exec     — baseline curls+runs the tool; evasion uses
#                                 `ordo send --exec` (only tool execve seen)
#                      download — transfer only, no execution
#   --target <host>    SSH alias to run against (default: local)
#   --events <list>    tracee --events (default: execve,execveat,security_socket_connect)
#   --query <list>     comma substrings counted as "downloader" hits
#                      (default: curl,wget)
#   --port <n>         http.server port on target (default: 18777)
#   --ordo <path>      ordo binary (default: ./target/release/ordo)
#   -h, --help
#
# Examples:
#   bash observability/poc.sh                      # local, exec scenario
#   bash observability/poc.sh --scenario download
#   bash observability/poc.sh --target dev-vm-2    # remote over ssh

set -euo pipefail

# Persist full output of every run (screenshots/slides source material)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec > >(tee "$SCRIPT_DIR/poc-last-run.log") 2>&1

SCENARIO="exec"
TARGET="local"
TRACEE_EVENTS="execve,execveat,security_socket_connect"
QUERY="curl,wget"
PORT=18777
ORDO="${ORDO:-./target/release/ordo}"

DEST="/tmp/ordo-poc-tool"
TOOL_NAME="ordo-poc-tool"
SRC_DIR=""
TRACEE_CONTAINER=""

bold=$'\e[1m'
dim=$'\e[2m'
green=$'\e[32m'
yellow=$'\e[33m'
red=$'\e[31m'
reset=$'\e[0m'

usage() {
	sed -n '/^# Usage:/,/^[^#]/{ /^#/{ s/^# \?//; p } }' "$0"
	exit 0
}
die() {
	echo "${red}error:${reset} $*" >&2
	exit 1
}
say() { echo "${bold}==>${reset} $*"; }
info() { echo "    $*"; }

while [[ $# -gt 0 ]]; do
	case "$1" in
	--scenario)
		SCENARIO="$2"
		shift 2
		;;
	--target)
		TARGET="$2"
		shift 2
		;;
	--events)
		TRACEE_EVENTS="$2"
		shift 2
		;;
	--query)
		QUERY="$2"
		shift 2
		;;
	--port)
		PORT="$2"
		shift 2
		;;
	--ordo)
		ORDO="$2"
		shift 2
		;;
	-h | --help) usage ;;
	*) die "Unknown option: $1" ;;
	esac
done

case "$SCENARIO" in exec | download) ;; *) die "Unknown --scenario '$SCENARIO'. Valid: exec download" ;; esac

# ── Target command wrapper ───────────────────────────────────────────────────
# run "<cmd>" executes on the target (local shell or via ssh).
run() {
	if [[ "$TARGET" == "local" ]]; then
		bash -c "$1"
	else
		ssh "$TARGET" "$1"
	fi
}

SUDO="sudo"
[[ "$(run 'id -u')" == "0" ]] && SUDO=""

# ── Prerequisites ────────────────────────────────────────────────────────────
[[ -x "$ORDO" ]] || die "ordo binary not found at $ORDO (cargo build --release)"
run "command -v docker >/dev/null" || die "docker not found on target ($TARGET)"
run "command -v python3 >/dev/null" || die "python3 not found on target ($TARGET)"

if [[ "$TARGET" == "local" ]]; then
	BACKEND="exec:sh"
else
	BACKEND="ssh:$TARGET"
	run "command -v perl >/dev/null" || info "note: perl missing on target — chmod falls back to /bin/chmod"
fi

# ── Toy tool (the "downloaded malware": prints id, harmless) ────────────────
SRC_DIR=$(mktemp -d /tmp/ordo-poc.XXXXXX)
TOOL_SRC="$SRC_DIR/tool.sh"
cat >"$TOOL_SRC" <<'EOF'
#!/bin/sh
echo "[tool] ran as $(id -un)@$(hostname)"
EOF

cleanup() {
	[[ -n "$TRACEE_CONTAINER" ]] && run "$SUDO docker kill '$TRACEE_CONTAINER' >/dev/null 2>&1" || true
	run "pkill -f 'http.server $PORT' >/dev/null 2>&1" || true
	run "rm -f '$DEST' /tmp/$TOOL_NAME-src.sh" || true
	rm -rf "$SRC_DIR"
}
trap cleanup EXIT

# ── Tracee helpers (start/stop + filtered event dump) ───────────────────────
TRACEE_LOG=""

start_tracee() {
	TRACEE_LOG="/tmp/tracee-ordo-$$-$RANDOM.log"
	TRACEE_CONTAINER="tracee-ordo-$$-$RANDOM"
	say "starting tracee on $TARGET (events=$TRACEE_EVENTS)"
	# kernel headers mounts are best-effort: hosts with BTF (/sys/kernel/btf)
	# need neither; mount only what exists (NixOS has no /usr/src)
	local vols="-v /etc/os-release:/etc/os-release-host:ro"
	for d in /lib/modules /usr/src /boot; do
		run "test -d $d" && vols="$vols -v $d:$d:ro"
	done
	run "$SUDO docker run --rm --name '$TRACEE_CONTAINER' \
		--privileged --pid=host --cgroupns=host \
		$vols \
		aquasec/tracee:latest \
		--output json --events '$TRACEE_EVENTS' \
		>'$TRACEE_LOG' 2>&1 &"
	sleep 8 # eBPF attach takes seconds; shorter loses events
	run "$SUDO docker ps --filter name='$TRACEE_CONTAINER' --format '{{.Names}}'" | grep -q . ||
		{
			run "cat '$TRACEE_LOG'" || true
			die "tracee container died on $TARGET"
		}
}

# stop_tracee: kills tracee, saves event log locally for repeated filtering.
stop_tracee() {
	sleep 2 # let tracee flush
	run "$SUDO docker kill '$TRACEE_CONTAINER' >/dev/null 2>&1" || true
	TRACEE_CONTAINER=""
	run "cat '$TRACEE_LOG' 2>/dev/null" >"$SRC_DIR/tracee-events.json" || true
	run "rm -f '$TRACEE_LOG'" || true
}

# filter_events <query>: prints matching events from the saved log,
# sets LAST_EVENT_COUNT. Empty query matches everything.
filter_events() {
	local q="$1"
	local out
	out=$(python3 -c "
import sys, json
query = [q.strip().lower() for q in '$q'.split(',') if q.strip()]
found = 0
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        e = json.loads(line)
    except Exception:
        continue
    if 'eventName' not in e:
        continue
    haystack = json.dumps(e).lower()
    if query and not any(q in haystack for q in query):
        continue
    evt  = e.get('eventName', '?')
    proc = e.get('processName', '?')
    args = e.get('args') or []
    argv = next((a.get('value') for a in args
                 if isinstance(a, dict) and a.get('name') == 'argv'), None)
    cmdline = ' '.join(str(x) for x in argv) if argv else str(next(
        (a.get('value') for a in args
         if isinstance(a, dict) and a.get('name') == 'pathname'), ''))
    if not cmdline:
        ra = next((a.get('value') for a in args
                   if isinstance(a, dict) and a.get('name') == 'remote_addr'), None)
        if isinstance(ra, dict):
            cmdline = '%s:%s' % (ra.get('sin_addr', ra.get('sin6_addr', '?')),
                                 ra.get('sin_port', ra.get('sin6_port', '?')))
    if found < 30:
        print(f'  {evt:<24} {proc:<14} {cmdline}'[:160].rstrip())
    found += 1
if found > 30:
    print(f'  ... {found - 30} more')
print(f'COUNT={found}')
" <"$SRC_DIR/tracee-events.json" 2>/dev/null || echo "COUNT=0")
	LAST_EVENT_COUNT=$(echo "$out" | grep -o 'COUNT=[0-9]*' | tail -1 | cut -d= -f2)
	LAST_EVENT_COUNT="${LAST_EVENT_COUNT:-0}"
	echo "$out" | grep -v '^COUNT=' || true
	if [[ "$LAST_EVENT_COUNT" -eq 0 ]]; then
		echo "  (none matched${q:+ query='$q'})"
	fi
	return 0
}

# ── Plan ─────────────────────────────────────────────────────────────────────
echo ""
echo "${bold}ordo observability PoC — tracee: downloader vs ordo${reset}"
echo "  target:   ${green}${TARGET}${reset}"
echo "  scenario: ${yellow}${SCENARIO}${reset}"
echo "  backend:  ${yellow}${BACKEND}${reset}"
echo "  events:   ${dim}${TRACEE_EVENTS}${reset}"
echo "  query:    ${dim}${QUERY}${reset} (downloader indicators)"
echo ""

# ── Setup: http server on target serving the tool ────────────────────────────
say "serving tool via python http.server on $TARGET :$PORT (setup, outside capture)"
if [[ "$TARGET" == "local" ]]; then
	cp "$TOOL_SRC" "/tmp/$TOOL_NAME-src.sh"
else
	scp -q "$TOOL_SRC" "$TARGET:/tmp/$TOOL_NAME-src.sh"
fi
run "nohup python3 -m http.server $PORT --bind 127.0.0.1 --directory /tmp >/dev/null 2>&1 &"
sleep 1
run "curl -fsS -o /dev/null http://127.0.0.1:$PORT/$TOOL_NAME-src.sh" ||
	die "http.server not reachable on $TARGET"

# ── Baseline: classic curl/wget download (+chmod +run) ──────────────────────
say "Baseline: curl download on $TARGET (no ordo)"
start_tracee
info "curl -fsSL http://127.0.0.1:$PORT/$TOOL_NAME-src.sh -o $DEST ..."
if [[ "$SCENARIO" == "exec" ]]; then
	run "curl -fsSL http://127.0.0.1:$PORT/$TOOL_NAME-src.sh -o $DEST && chmod +x $DEST && $DEST"
else
	run "curl -fsSL http://127.0.0.1:$PORT/$TOOL_NAME-src.sh -o $DEST"
fi
echo ""
say "Baseline tracee events (query=$QUERY)"
stop_tracee
filter_events "$QUERY"
BASELINE_COUNT="$LAST_EVENT_COUNT"
run "rm -f '$DEST'" || true
echo ""

# ── Evasion: ordo transfer ───────────────────────────────────────────────────
say "Evasion: ordo send -t $BACKEND -r $DEST $([ "$SCENARIO" == exec ] && echo --exec)"
start_tracee
EXEC_FLAG=""
[[ "$SCENARIO" == "exec" ]] && EXEC_FLAG="--exec"
# ordo's informational stderr ("chmod 0755 ...", "exec: ...") is filtered out:
# next to the tracee event list it reads as caught events. Real errors still pass.
"$ORDO" send "$TOOL_SRC" -t "$BACKEND" -r "$DEST" $EXEC_FLAG 2>&1 |
	grep -v -e '^chmod 0[0-9]* ' -e '^exec: "' | tail -5 || true
echo ""
say "Evasion tracee events (query=$QUERY)"
stop_tracee
filter_events "$QUERY"
EVASION_COUNT="$LAST_EVENT_COUNT"
echo ""

# ── Summary ──────────────────────────────────────────────────────────────────
say "Comparison (downloader events matching '$QUERY' on $TARGET)"
echo "  baseline (curl):  ${yellow}${BASELINE_COUNT}${reset} event(s)"
echo "  evasion  (ordo):  ${yellow}${EVASION_COUNT}${reset} event(s)"
echo ""
if [[ "$BASELINE_COUNT" -eq 0 ]]; then
	echo "${yellow}WARN${reset} baseline saw 0 downloader events — tracee hooks inactive? Result meaningless."
	exit 1
elif [[ "$EVASION_COUNT" -eq 0 ]]; then
	echo "${green}PASS${reset} — tracee saw the curl/wget baseline but no downloader activity during the ordo transfer."
else
	echo "${red}FAIL${reset} — ordo run produced $EVASION_COUNT downloader-matching event(s)."
	exit 1
fi
