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
"$bin" synthesize "$tmp/project/index.jsonl" --out "$tmp/project/index.json" >/dev/null
"$bin" validate "$tmp/project/index.json" >/dev/null
python3 - "$tmp/project" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1])
records=[json.loads(line) for line in (root/'index.jsonl').read_text().splitlines()]
manifest=[json.loads(line) for line in (root/'analysis/scan-manifest.jsonl').read_text().splitlines()]
assets=[r for r in records if r['record_type']=='artifact' and r.get('parent_asset_id') is None]
children=[r for r in records if r['record_type']=='artifact' and r.get('parent_asset_id')]
aliases=[r for r in records if r['record_type']=='artifact_path']
assert len(assets)==1 and len(children)>=2 and aliases, records
assert len(manifest)==1 and manifest[0]['artifact_id']==assets[0]['artifact_id']
assert manifest[0]['probe_artifact_id'] in {r['artifact_id'] for r in children}
assert set(manifest[0]['frame_artifact_ids']).issubset({r['artifact_id'] for r in children}|{r['artifact_id'] for r in aliases})
probe=root/manifest[0]['probe']
assert json.loads(probe.read_text())['streams']
frames=list((root/'analysis'/f"{assets[0]['sha256']}-frames").glob('frame_*.jpg'))
assert len(frames)>=2, frames
for frame in [r for r in children if r['kind']=='frame']:
    assert frame['parent_asset_id']==assets[0]['artifact_id'] and frame['time']['at_ms'] is not None
index=json.loads((root/'index.json').read_text())
same_hash_frames=[a for a in index['artifacts'] if a['kind']=='frame' and len(a['occurrences'])>1]
assert same_hash_frames and {o['time']['at_ms'] for o in same_hash_frames[0]['occurrences']} >= {0,500}
print(f"scan E2E passed: {len(frames)} sampled frames registered as evidence")
PY
