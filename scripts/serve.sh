#!/usr/bin/env bash
# Server statis lokal untuk ./dist (WebAssembly butuh http://, bukan file://).
set -euo pipefail
cd "$(dirname "$0")/../dist"
PORT="${1:-8080}"
echo "Buka http://localhost:$PORT"
exec python3 -m http.server "$PORT"
