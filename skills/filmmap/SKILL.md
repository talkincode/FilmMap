---
name: filmmap
description: Use FilmMap to inventory media, declare available analysis capabilities, plan evidence gathering, and create searchable editing indexes. Trigger when organizing footage, finding shots by content/location/time, or preparing edit selects from local media.
---

# FilmMap media indexing workflow

1. Inspect the requested media root and preserve source files. Initialize a workspace with `filmmap init <workspace>`.
2. Run `filmmap profile detect --output <workspace>/profile.json`; validate profiles with `filmmap profile validate`.
3. Define a requirement JSON with a stable `id` and `required_capabilities`, then run `filmmap plan --requirement ... --profile ...`. Execute only capabilities marked available; ask the host Agent to provide unverified vision, OCR, transcription, or place inference.
4. For a media directory, use `filmmap-scan.sh <media-root> <workspace> [--frames] [interval]` for repeatable batch registration and FFprobe evidence. Use `--frames` when frame sampling is appropriate, then visually inspect the outputs. Increase sampling around scene changes or ambiguous content; state coverage and blind spots.
5. `filmmap-scan.sh` registers probe JSON and every sampled frame as child artifacts. Each frame keeps source `time_ms`; duplicate frame bytes share a content ID but retain separate `occurrences` in the canonical index.
6. Add observations with `filmmap observe add <index> <asset-id> --kind <kind> --class fact|measurement|interpretation --at-ms <ms>` or a half-open `--start-ms/--end-ms` range. Use `--value-json` for structured content and repeat `--evidence-ref <artifact-id>` for the frame/probe/transcript that supports the observation. Choose `--status accepted|candidate|rejected|conflicted`; do not upgrade inference to fact.
7. For location, use a `location_candidate` observation whose JSON records candidate labels/coordinates, basis, alternatives, direct-vs-contextual evidence, and uncertainty. GPS metadata, visible signage, contextual inference, and unknown location are separate outcomes.
8. Run `filmmap synthesize <workspace>/index.jsonl --out <workspace>/index.json`, then validate and query the canonical index with `--asset-id`, `--kind`, and millisecond bounds. Review synthesis issues for contradictory overlapping observations before using selects.
9. Prefer exact source time ranges, stable asset/artifact IDs, evidence pointers, confidence, and explicit unknowns. Do not describe simple text filtering as model-based semantic search.

FilmMap defines capability contracts and interoperable indexes. It does not bundle or silently invoke a vision, OCR, ASR, geocoding, or editing service. Use installed tools only when a capability profile says they are available, and report unavailable work as a gap.
