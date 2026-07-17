#!/usr/bin/env bash
# Run the transcription benchmark (wren-bench) against the golden set
# (tests/fixtures/golden).
#
# Usage: ./scripts/bench-stt.sh [--samples id1,id2] [--variants sem-vad,vad,vad+pausas]
#                               [--compress-over-ms N] [--quiet]
# Provider: the active one from settings.json; env overrides:
#   WREN_BENCH_BASE_URL, WREN_BENCH_MODEL, WREN_BENCH_API_KEY
#
# Warning: runs against the REAL provider (consumes quota; e.g. a 10 s minimum
# billing per request on Groq).

set -euo pipefail

cd "$(dirname "$0")/.."
exec cargo run -p wren-bench --release -- "$@"
