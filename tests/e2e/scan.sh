#!/usr/bin/env bash
set -euo pipefail
bin=${1:-target/debug/filmmap}
tools=${2:-tools}
command -v ffmpeg >/dev/null || { echo "ffmpeg is required for scan E2E" >&2; exit 127; }
command -v ffprobe >/dev/null || { echo "ffprobe is required for scan E2E" >&2; exit 127; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/media/nested"
ln -s "$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")" "$tmp/bin/filmmap"
for tool in filmmap-probe.sh filmmap-frames.sh filmmap-scan.sh; do ln -s "$(cd "$tools" && pwd)/$tool" "$tmp/bin/$tool"; done
ffmpeg -hide_banner -loglevel error -f lavfi -i 'color=c=blue:s=320x240:d=1' -c:v mpeg4 -y "$tmp/media/nested/test clip.mp4"
"$bin" init "$tmp/project"
PATH="$tmp/bin:$PATH" filmmap-scan.sh "$tmp/media" "$tmp/project" --frames 0.5
"$bin" validate "$tmp/project/index.jsonl"
python3 - "$tmp/project" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1])
records=[json.loads(line) for line in (root/'index.jsonl').read_text().splitlines()]
manifest=[json.loads(line) for line in (root/'analysis/scan-manifest.jsonl').read_text().splitlines()]
assert len(records)==1 and records[0]['record_type']=='artifact'
assert len(manifest)==1 and manifest[0]['artifact_id']==records[0]['artifact_id']
probe=root/manifest[0]['probe']
assert json.loads(probe.read_text())['streams']
frames=list((root/'analysis'/f"{records[0]['sha256']}-frames").glob('frame_*.jpg'))
assert len(frames)>=2, frames
print(f"scan E2E passed: {len(frames)} sampled frames")
PY
