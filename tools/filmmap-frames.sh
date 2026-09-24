#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 3 || $# -gt 4 ]]; then echo "usage: filmmap-frames.sh MEDIA_FILE OUTPUT_DIR INTERVAL_SECONDS [MAX_FRAMES]" >&2; exit 2; fi
media=$1; out=$2; interval=$3; limit=${4:-300}
command -v ffmpeg >/dev/null || { echo "ffmpeg is required" >&2; exit 127; }
command -v ffprobe >/dev/null || { echo "ffprobe is required" >&2; exit 127; }
[[ -f "$media" ]] || { echo "media file not found: $media" >&2; exit 1; }
[[ "$interval" =~ ^[0-9]+([.][0-9]+)?$ ]] && awk -v n="$interval" 'BEGIN{exit !(n>0)}' || { echo "interval must be > 0" >&2; exit 2; }
mkdir -p "$out"
duration=$(ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 -- "$media")
awk -v d="$duration" -v i="$interval" -v m="$limit" 'BEGIN{n=int(d/i)+1;if(n>m)n=m;for(k=0;k<n;k++)printf "%.3f\n",k*i}' |
while IFS= read -r ts; do
  [[ "$ts" == .* ]] && ts="0$ts"
  name=$(printf '%s' "$ts" | tr '.' '_')
  ffmpeg -hide_banner -loglevel error -ss "$ts" -i "$media" -frames:v 1 -vf 'scale=960:-2:out_range=pc,format=yuvj420p' -y "$out/frame_${name}.jpg"
  printf '{"time_seconds":%s,"frame":"frame_%s.jpg"}\n' "$ts" "$name"
done > "$out/frames.jsonl"
