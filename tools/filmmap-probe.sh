#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 1 ]]; then echo "usage: filmmap-probe.sh MEDIA_FILE" >&2; exit 2; fi
command -v ffprobe >/dev/null || { echo "ffprobe is required (install ffmpeg)" >&2; exit 127; }
exec ffprobe -v error -show_format -show_streams -of json -- "$1"
