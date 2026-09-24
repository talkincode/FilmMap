#!/usr/bin/env bash
set -euo pipefail
bin=${1:-target/debug/filmmap}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
root="$tmp/project"
"$bin" init "$root"
"$bin" init "$root"
"$bin" profile validate "$root/profile.json"
"$bin" capability list > "$tmp/catalog.json"
cat > "$tmp/requirement.json" <<'EOF'
{"id":"editing/location-and-visual","required_capabilities":["media.probe","image.describe","geolocation.infer"]}
EOF
"$bin" plan --requirement "$tmp/requirement.json" --profile "$root/profile.json" > "$tmp/plan.json"
python3 -c 'import json,sys;p=json.load(open(sys.argv[1]));assert isinstance(p["ready"],list);assert "image.describe" in p["unverified"];assert not p["executable"]' "$tmp/plan.json"
printf 'synthetic media bytes\n' > "$tmp/clip.mp4"
if "$bin" artifact add "$root/index.jsonl" "$tmp/missing.mp4" >/dev/null 2>&1; then echo "expected unreadable media to fail" >&2; exit 1; fi
test ! -s "$root/index.jsonl"
id=$("$bin" artifact add "$root/index.jsonl" "$tmp/clip.mp4")
cp "$tmp/clip.mp4" "$tmp/clip-copy.mp4"
alias_id=$("$bin" artifact add "$root/index.jsonl" "$tmp/clip-copy.mp4")
test "$alias_id" = "$id"
"$bin" observe add "$root/index.jsonl" "$id" --capability image.describe --at 12.5 --value 'person walking beside a lake' --source human-review --confidence 0.94
"$bin" validate "$root/index.jsonl"
"$bin" query "$root/index.jsonl" lake --from 12 --to 13 > "$tmp/results.json"
python3 -c 'import json,sys;r=json.load(open(sys.argv[1]));assert len(r)==1 and r[0]["time_seconds"]==12.5' "$tmp/results.json"
"$bin" install skill --dir "$tmp/skills/filmmap"
"$bin" install tools --dir "$tmp/tools"
test -x "$tmp/tools/filmmap-frames.sh"
test -x "$tmp/tools/filmmap-scan.sh"
printf 'custom user file\n' > "$tmp/skills/filmmap/SKILL.md"
if "$bin" install skill --dir "$tmp/skills/filmmap" >/dev/null 2>&1; then echo "expected install conflict to fail" >&2; exit 1; fi
grep -q 'custom user file' "$tmp/skills/filmmap/SKILL.md"
echo "FilmMap CLI E2E passed"
