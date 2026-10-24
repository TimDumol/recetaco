#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build
./target/debug/recetaco &
server_pid=$!
trap 'kill -INT "$server_pid" 2>/dev/null || true' EXIT INT TERM
npm --prefix web run dev
