#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: filmmap-scan.sh MEDIA_ROOT WORKSPACE [--frames] [INTERVAL_SECONDS]" >&2
  exit 2
fi
root=$1
workspace=$2
frames=false
interval=30
for arg in "${@:3}"; do
  if [[ "$arg" == "--frames" ]]; then frames=true; else interval=$arg; fi
done
[[ -d "$root" ]] || { echo "media root not found: $root" >&2; exit 1; }
[[ -f "$workspace/filmmap.json" && -f "$workspace/index.jsonl" ]] || { echo "initialize workspace first: filmmap init $workspace" >&2; exit 1; }
command -v filmmap >/dev/null || { echo "filmmap CLI is required on PATH" >&2; exit 127; }
command -v ffprobe >/dev/null || { echo "ffprobe is required (install ffmpeg)" >&2; exit 127; }
probe_script=$(dirname "$0")/filmmap-probe.sh
frames_script=$(dirname "$0")/filmmap-frames.sh
mkdir -p "$workspace/analysis"
manifest="$workspace/analysis/scan-manifest.jsonl"
: > "$manifest"
count=0
while IFS= read -r -d '' media; do
  ext=$(printf '%s' "${media##*.}" | tr '[:upper:]' '[:lower:]')
  case "$ext" in mp4|mov|m4v|mkv|avi|mts|m2ts|mpg|mpeg|3gp) ;; *) continue ;; esac
  artifact_id=$(filmmap artifact add "$workspace/index.jsonl" "$media" --kind video)
  key=${artifact_id#sha256:}
  "$probe_script" "$media" > "$workspace/analysis/$key.probe.json"
  probe_artifact_id=$(filmmap artifact add "$workspace/index.jsonl" "$workspace/analysis/$key.probe.json" --kind probe-result --parent-asset "$artifact_id" --mime-type application/json)
  frame_ids_json='[]'
  if [[ "$frames" == true ]]; then
    "$frames_script" "$media" "$workspace/analysis/$key-frames" "$interval" >/dev/null
    frame_ids_json=$(python3 - "$workspace/index.jsonl" "$artifact_id" "$workspace/analysis/$key-frames/frames.jsonl" <<'PY'
import json, pathlib, subprocess, sys
index, parent, manifest = sys.argv[1:]
ids=[]
for line in pathlib.Path(manifest).read_text().splitlines():
    entry=json.loads(line)
    path=pathlib.Path(manifest).parent / entry["frame"]
    ms=round(float(entry["time_seconds"])*1000)
    ids.append(subprocess.check_output(["filmmap","artifact","add",index,str(path),"--kind","frame","--parent-asset",parent,"--time-ms",str(ms),"--mime-type","image/jpeg"],text=True).strip())
print(json.dumps(ids))
PY
)
  fi
  python3 - "$artifact_id" "$media" "$key.probe.json" "$probe_artifact_id" "$frame_ids_json" "$frames" <<'PY' >> "$manifest"
import json, sys
print(json.dumps({"artifact_id":sys.argv[1],"path":sys.argv[2],"probe":"analysis/"+sys.argv[3],"probe_artifact_id":sys.argv[4],"frame_artifact_ids":json.loads(sys.argv[5]),"frames_requested":sys.argv[6]=="true"},ensure_ascii=False))
PY
  count=$((count + 1))
done < <(find "$root" -type f -print0)
printf 'Scanned %s video files. Probe evidence: %s\n' "$count" "$workspace/analysis"
