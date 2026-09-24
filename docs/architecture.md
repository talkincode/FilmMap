# Architecture and capability contracts

```text
media files (read-only)
   ├── FFmpeg / metadata scripts ──┐
   └── host Agent / external AI ───┤ evidence + observations
                                   ▼
FilmMap CLI: profile → plan → stable artifact IDs → validate/query/export
                                   ▼
JSONL evidence index + portable profile (editing handoff)
```

## Profile capability set

`capabilities/catalog.json` is the stable vocabulary. Each capability declares an ID, purpose, outputs and optional executable used only for local detection. A `Capability Profile` says which capabilities are `available`, `unavailable`, `unverified`, or `failed` in a specific environment. `available` requires a usable provider/executable; `unverified` means the capability can be supplied by an Agent/service but FilmMap did not verify it. Profiles should add provider/version, input limits, precision, languages, privacy and provenance details as they evolve. Never infer availability from a package name alone.

| Profile family | Capability IDs | Outputs used by editors |
| --- | --- | --- |
| Media facts | `media.probe`, `media.scene.detect`, `asset.hash.sha256` | duration, streams, shot boundaries, stable identity |
| Visual sampling | `media.frame.extract`, `media.contact_sheet`, `image.describe`, `image.ocr` | time-mapped frames, descriptions, visible text |
| Audio | `audio.extract`, `audio.transcribe` | source-aligned speech segments |
| Capture location | `metadata.exif.read`, `metadata.gps.read`, `geolocation.infer` | camera facts, embedded coordinates, candidate place with evidence |

These names define capability semantics, not service implementations. The plan command compares a requirement's `required_capabilities` with this profile and explicitly separates ready, missing and unverified items.

## Evidence and time

An artifact is a source file identified by SHA-256. Observations reference the artifact and preserve capability/source, optional source timestamp, confidence and value. Preserve original media; generated frames and audio derivatives are artifacts too. Timestamps are source-media seconds unless an explicit offset is recorded. Location claims include evidence and alternatives; embedded coordinates are not interchangeable with visual inference. Contradictions should remain visible.

## Agent extensibility

The bundled skill describes when and how to use FilmMap. The `tools/` manifest provides optional scripts. Agents choose them based on the active profile and task; FilmMap does not install packages, invoke network AI, mutate source media, or silently choose an external provider.
