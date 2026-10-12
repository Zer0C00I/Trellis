#!/usr/bin/env bash
# Builds demo/out/demo.mp4 from the repository root: kiosk frames, then a live recording of the stage.
set -euo pipefail
cd "$(dirname "$0")/.."
node --test demo/test/*.test.mjs
node demo/capture-kiosk.mjs
node demo/record.mjs
ffprobe -v error -show_entries format=duration -of csv=p=0 demo/out/demo.mp4
