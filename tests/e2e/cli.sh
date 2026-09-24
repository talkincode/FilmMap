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
{"id":"editing/location-and-visual","required_capabilities":["media.probe"],"targets":[{"capability":"image.describe","level":"required","target_precision_ms":250}],"if_available_capabilities":["geolocation.infer"],"constraints":{"external_processing":false}}
EOF
"$bin" plan --requirement "$tmp/requirement.json" --profile "$root/profile.json" > "$tmp/plan.json"
python3 -c 'import json,sys;p=json.load(open(sys.argv[1]));assert p["status"]=="potentially_satisfied";assert "image.describe" in p["required"]["potentially_satisfied"];assert "geolocation.infer" in p["if_available"]["potentially_satisfied"];assert {c["constraint"] for c in p["constraint_checks"]}=={"target_precision_ms","external_processing"};assert not p["executable"]' "$tmp/plan.json"
printf 'synthetic media bytes\n' > "$tmp/clip.mp4"
if "$bin" artifact add "$root/index.jsonl" "$tmp/missing.mp4" >/dev/null 2>&1; then echo "expected unreadable media to fail" >&2; exit 1; fi
test ! -s "$root/index.jsonl"
id=$("$bin" artifact add "$root/index.jsonl" "$tmp/clip.mp4")
cp "$tmp/clip.mp4" "$tmp/clip-copy.mp4"
alias_id=$("$bin" artifact add "$root/index.jsonl" "$tmp/clip-copy.mp4")
test "$alias_id" = "$id"
python3 - "$tmp/frame.jpg" <<'PY'
import base64,sys
open(sys.argv[1],"wb").write(base64.b64decode("/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////2wBDAf//////////////////////////////////////////////////////////////////////////////////////wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIQAxAAAAF//8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQABBQJ//8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAgBAwEBPwF//8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAgBAgEBPwF//8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQAGPwJ//8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQABPyF//9oADAMBAAIAAwAAABAf/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAgBAwEBPxB//8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAgBAgEBPxB//8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQABPxB//9k="))
PY
frame_id=$("$bin" artifact add "$root/index.jsonl" "$tmp/frame.jpg" --kind frame --parent-asset "$id" --time-ms 12500 --mime-type image/jpeg)
if "$bin" observe add "$root/index.jsonl" "$id" --kind scene_description --value missing --evidence-ref sha256:0000000000000000000000000000000000000000000000000000000000000000 >/dev/null 2>&1; then echo "expected a broken evidence reference to fail" >&2; exit 1; fi
"$bin" observe add "$root/index.jsonl" "$id" --kind scene_description --class interpretation --start-ms 12000 --end-ms 13000 --value-json '{"text":"blue interior floor and people wearing shoes"}' --source human-review --confidence 0.94 --evidence-ref "$frame_id"
"$bin" observe add "$root/index.jsonl" "$id" --kind scene_description --class interpretation --start-ms 12000 --end-ms 13000 --value-json '{"text":"outdoor lakeside walkway"}' --source human-review --confidence 0.4 --evidence-ref "$frame_id"
"$bin" observe add "$root/index.jsonl" "$id" --kind location_candidate --class interpretation --at-ms 12500 --value-json '{"candidates":[{"label":"unknown indoor venue","basis":"visual resemblance only"}],"alternatives":["unknown location"],"direct_evidence":false}' --source human-review --status candidate --confidence 0.3 --evidence-ref "$frame_id"
"$bin" observe add "$root/index.jsonl" "$id" --kind scene_description --class interpretation --start-ms 12000 --end-ms 13000 --value-json '{"text":"blue interior floor and people wearing shoes"}' --source human-review --confidence 0.94 --evidence-ref "$frame_id"
"$bin" validate "$root/index.jsonl"
"$bin" synthesize "$root/index.jsonl" --out "$root/index.json"
"$bin" validate "$root/index.json"
"$bin" query "$root/index.json" scene --asset-id "$id" --kind scene_description --from-ms 12000 --to-ms 13000 > "$tmp/results.json"
python3 -c 'import json,sys;r=json.load(open(sys.argv[1]));assert len(r)==4 and sum(x["id"].startswith("seg_sha256:") for x in r)==2 and all(x["time"]["start_ms"]==12000 for x in r)' "$tmp/results.json"
python3 - "$root/index.json" "$id" "$frame_id" <<'PY'
import json,sys
i=json.load(open(sys.argv[1]));assert i["revision"].startswith("sha256:") and len(i["segments"])==2
assert any(x["code"]=="OBSERVATION_CONFLICT" for x in i["issues"])
assert all(set(o["evidence_refs"]).issubset({a["id"] for a in i["artifacts"]}) for o in i["observations"])
assert any(o["kind"]=="location_candidate" and o["asset_id"]==sys.argv[2] for o in i["observations"])
assert sys.argv[3] in {a["id"] for a in i["artifacts"]}
PY
"$bin" install skill --dir "$tmp/skills/filmmap"
"$bin" install tools --dir "$tmp/tools"
test -x "$tmp/tools/filmmap-frames.sh"
test -x "$tmp/tools/filmmap-scan.sh"
printf 'custom user file\n' > "$tmp/skills/filmmap/SKILL.md"
if "$bin" install skill --dir "$tmp/skills/filmmap" >/dev/null 2>&1; then echo "expected install conflict to fail" >&2; exit 1; fi
grep -q 'custom user file' "$tmp/skills/filmmap/SKILL.md"
echo "FilmMap CLI E2E passed"
