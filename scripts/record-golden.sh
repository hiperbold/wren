#!/usr/bin/env bash
# Record a personal sample of the golden set (tests/fixtures/golden/personal).
#
# Usage: ./scripts/record-golden.sh <id>          (e.g. ditado-prompt)
#        ./scripts/record-golden.sh --list        list the available scripts
#
# Shows the script (<id>.roteiro.md), records from the default microphone as
# 16-bit mono 48 kHz WAV, and saves it as personal/<id>.wav — the format
# wren-bench expects (it resamples to 16 kHz itself, just like the app).
# End the recording with Ctrl+C (arecord closes the WAV cleanly).

set -euo pipefail

cd "$(dirname "$0")/.."
dir=tests/fixtures/golden/personal

if [[ "${1:-}" == "--list" || -z "${1:-}" ]]; then
    echo "Available scripts:"
    for r in "$dir"/*.roteiro.md; do
        id="$(basename "$r" .roteiro.md)"
        [[ -f "$dir/$id.wav" ]] && status="✅ recorded" || status="⏳ pending"
        echo "  $id — $status"
    done
    echo
    echo "Usage: ./scripts/record-golden.sh <id>"
    exit 0
fi

id="$1"
script="$dir/$id.roteiro.md"
wav="$dir/$id.wav"

if [[ ! -f "$script" ]]; then
    echo "error: script not found: $script" >&2
    echo "(run with --list to see the available ones)" >&2
    exit 1
fi

if [[ -f "$wav" ]]; then
    # Golden-set rule: an existing sample never changes (historical series).
    read -rp "$wav already exists — overwrite? [y/N] " reply
    [[ "$reply" == "y" || "$reply" == "Y" ]] || exit 0
fi

command -v arecord >/dev/null || {
    echo "error: arecord (alsa-utils) not found" >&2
    exit 1
}

echo "════════════════════════════════════════════════════════════════════"
cat "$script"
echo "════════════════════════════════════════════════════════════════════"
echo
read -rp "Enter to start recording (Ctrl+C stops the recording)… "
echo

arecord -f S16_LE -r 48000 -c 1 "$wav" || true

echo
echo "Saved to $wav"
echo "Check it by listening:  aplay $wav"
echo "Then run:               ./scripts/bench-stt.sh --samples $id"
