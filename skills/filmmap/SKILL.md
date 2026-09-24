---
name: filmmap
description: Use FilmMap to inventory media, declare available analysis capabilities, plan evidence gathering, and create searchable editing indexes. Trigger when organizing footage, finding shots by content/location/time, or preparing edit selects from local media.
---

# FilmMap media indexing workflow

1. Inspect the requested media root and preserve source files. Initialize a workspace with `filmmap init <workspace>`.
2. Run `filmmap profile detect --output <workspace>/profile.json`; validate profiles with `filmmap profile validate`.
3. Define a requirement JSON with a stable `id` and `required_capabilities`, then run `filmmap plan --requirement ... --profile ...`. Execute only capabilities marked available; ask the host Agent to provide unverified vision, OCR, transcription, or place inference.
4. For a media directory, use `filmmap-scan.sh <media-root> <workspace> [--frames] [interval]` for repeatable batch registration and FFprobe evidence. Use `--frames` when frame sampling is appropriate, then visually inspect the outputs. Increase sampling around scene changes or ambiguous content; state coverage and blind spots.
5. Add source files using `filmmap artifact add <index> <media>`. Add observations using `filmmap observe add` with capability, source, timestamp, confidence, and a short factual value. Location observations must keep evidence and alternatives; never present an inference as embedded GPS truth.
6. Validate and query the index. Prefer exact source time ranges, stable artifact IDs, evidence pointers, confidence, and explicit unknowns. Keep claims separate from evidence and preserve conflicting observations.

FilmMap defines capability contracts and interoperable indexes. It does not bundle or silently invoke a vision, OCR, ASR, geocoding, or editing service. Use installed tools only when a capability profile says they are available, and report unavailable work as a gap.
