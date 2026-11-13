#!/bin/sh
# Builds this checkout's mods into the Horizon launcher's selected instance (macOS/Linux dev loop).
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SET="$ROOT/target/build-set"
rm -rf "$SET" && mkdir -p "$SET"
for m in $(python3 -c "import json;print(' '.join(json.load(open('$ROOT/horizon.json'))['mods']))"); do
  rsync -a --exclude target --exclude Cargo.lock "$ROOT/$m" "$SET/"
done
L="${HORIZON_LAUNCHER:-$HOME/Applications/Horizon.app/Contents/MacOS/horizon-launcher}"
exec "$L" native-build "$SET" --trust-native-code "$@"
