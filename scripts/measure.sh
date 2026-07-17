#!/usr/bin/env bash
# Measure Wren's RSS per state.
# Usage: start the app and run ./scripts/measure.sh — it sums the RSS of all
# app processes (Rust + webviews) and compares against the per-state budgets.
#
# Manual, assisted measurement. Automating the states (triggering a session,
# opening/closing the UI, 10 anti-leak cycles) will come with the MVP.

set -euo pipefail

MAX_S0_KB=$((30 * 1024))

pids=$(pgrep -f '[w]ren-desktop' || true)
if [[ -z "$pids" ]]; then
  echo "wren-desktop is not running." >&2
  exit 1
fi

total_kb=0
echo "--- processes ---"
for pid in $pids; do
  rss=$(ps -o rss= -p "$pid" | tr -d ' ')
  cmd=$(ps -o comm= -p "$pid")
  echo "  pid=$pid rss=$((rss / 1024))MB ($cmd)"
  total_kb=$((total_kb + rss))
done

# WebKitGTK webviews are child processes (WebKitWebProcess etc.)
for pid in $(pgrep -f 'WebKit.*Process' || true); do
  parent=$(ps -o ppid= -p "$pid" | tr -d ' ')
  if echo "$pids" | grep -q "$parent"; then
    rss=$(ps -o rss= -p "$pid" | tr -d ' ')
    echo "  pid=$pid rss=$((rss / 1024))MB (webview)"
    total_kb=$((total_kb + rss))
  fi
done

echo "--- total ---"
echo "Total RSS: $((total_kb / 1024)) MB"

echo "(state S0 expected to be <= $((MAX_S0_KB / 1024)) MB when no window is open)"

if [[ $total_kb -le $MAX_S0_KB ]]; then
  echo "OK for S0 ✓"
else
  echo "Above the S0 maximum — check for an open window (S1/S2 have their own budget)."
fi
