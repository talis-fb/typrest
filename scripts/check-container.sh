#!/usr/bin/env bash
# Builds the Typrest container image and checks it over HTTP.
#
# Phase 1 runs the image's bundled examples/minimal project and checks that a stop exits with
# status 0 (Typrest runs as PID 1 and must handle SIGTERM itself).
# Phase 2 mounts a temporary copy of examples/invoice at /project.
#
# Environment:
#   CONTAINER_ENGINE  docker (default) or podman
#   CHECK_IMAGE       image tag to build (default typrest:check)
#   CHECK_PORT        host port on 127.0.0.1 (default: a free port)
#   CHECK_TIMEOUT     seconds to wait for /_healthz (default 60)
set -euo pipefail

ENGINE="${CONTAINER_ENGINE:-docker}"
IMAGE="${CHECK_IMAGE:-typrest:check}"
TIMEOUT="${CHECK_TIMEOUT:-60}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

WORK_DIR="$(mktemp -d)"
CONTAINERS=()

cleanup() {
    local container
    for container in "${CONTAINERS[@]}"; do
        "$ENGINE" rm -f "$container" >/dev/null 2>&1 || true
    done
    # Files created by the container user may not be removable by the host user under rootless
    # engines, so remove them from inside a container first.
    if [[ -d "$WORK_DIR/project" ]]; then
        "$ENGINE" run --rm --user 0 -v "$WORK_DIR:/work:Z" --entrypoint /bin/rm "$IMAGE" \
            -rf /work/project >/dev/null 2>&1 || true
    fi
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

pass() {
    echo "PASS: $*"
}

free_port() {
    python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()'
}

PORT="${CHECK_PORT:-$(free_port)}"
BASE_URL="http://127.0.0.1:$PORT"

wait_healthy() {
    local container="$1" body deadline
    deadline=$((SECONDS + TIMEOUT))
    while ((SECONDS < deadline)); do
        body="$(curl -fsS "$BASE_URL/_healthz" 2>/dev/null || true)"
        if [[ "$body" == '{"status":"ok"}' ]]; then
            pass "$container is healthy"
            return 0
        fi
        if [[ "$("$ENGINE" inspect -f '{{.State.Running}}' "$container" 2>/dev/null)" != "true" ]]; then
            break
        fi
        sleep 1
    done
    echo "--- logs for $container ---" >&2
    "$ENGINE" logs "$container" >&2 || true
    fail "$container did not report {\"status\":\"ok\"} on /_healthz within ${TIMEOUT}s"
}

# post_json <route> <data> <expected-status> [expect-pdf]
post_json() {
    local route="$1" data="$2" expected="$3" expect_pdf="${4:-}" out status
    out="$(mktemp "$WORK_DIR/response.XXXXXX")"
    status="$(curl -sS -o "$out" -w '%{http_code}' -X POST "$BASE_URL$route" \
        -H 'content-type: application/json' --data "$data")"
    if [[ "$status" != "$expected" ]]; then
        echo "--- response body ---" >&2
        head -c 2000 "$out" >&2 || true
        echo >&2
        fail "POST $route returned $status, expected $expected"
    fi
    if [[ -n "$expect_pdf" && "$(head -c 4 "$out")" != "%PDF" ]]; then
        fail "POST $route returned $status but the body is not a PDF"
    fi
    pass "POST $route -> $status${expect_pdf:+ (PDF, $(wc -c <"$out") bytes)}"
}

run_container() {
    local name="$1"
    shift
    CONTAINERS+=("$name")
    "$ENGINE" rm -f "$name" >/dev/null 2>&1 || true
    "$ENGINE" run -d --name "$name" -p "127.0.0.1:$PORT:8080" "$@" "$IMAGE" >/dev/null
}

stop_container() {
    "$ENGINE" rm -f "$1" >/dev/null 2>&1 || true
}

# check_graceful_stop <container>: stops a running container and checks it exits with status 0.
# Without a SIGTERM handler, PID 1 ignores the signal and the engine kills it, giving 137.
check_graceful_stop() {
    local container="$1" exit_code
    "$ENGINE" stop "$container" >/dev/null
    exit_code="$("$ENGINE" inspect -f '{{.State.ExitCode}}' "$container")"
    [[ "$exit_code" == "0" ]] || fail "$container exited with status $exit_code after stop, expected 0"
    pass "$container stopped with exit status 0"
}

echo "==> Building $IMAGE with $ENGINE"
"$ENGINE" build -t "$IMAGE" .

echo "==> Phase 1: bundled examples/minimal project"
MINIMAL="typrest-check-minimal-$$"
run_container "$MINIMAL"
wait_healthy "$MINIMAL"
post_json /hello "@examples/minimal/templates/hello/payload.json" 200 pdf
post_json /hello '{"greeting":"Hi"}' 422
check_graceful_stop "$MINIMAL"
stop_container "$MINIMAL"

echo "==> Phase 2: mounted examples/invoice project"
# Mount a world-writable copy so the container user can create /project/.typrest regardless of
# host ownership, and so the check leaves no generated files in the repository.
cp -R examples/invoice "$WORK_DIR/project"
chmod -R a+rwX "$WORK_DIR/project"
INVOICE="typrest-check-invoice-$$"
run_container "$INVOICE" -v "$WORK_DIR/project:/project:Z"
wait_healthy "$INVOICE"
post_json /invoice "@examples/invoice/templates/invoice/payload.json" 200 pdf
stop_container "$INVOICE"

echo "PASS: container check completed with $ENGINE"
