# Architecture and capability contracts

```text
media files (read-only)
   ├── FFmpeg / metadata scripts ──┐
   └── host Agent / external AI ───┤ evidence + observations
                                   ▼
FilmMap CLI: profile → plan → stable artifact IDs → evidence observations → canonical synthesize/validate/query
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

These names define capability semantics, not service implementations. The plan command classifies required, optional and if-available capabilities and reports precision/privacy constraint checks as satisfied, missing or unverified; it does not execute the model or benchmark output quality.

## Evidence and time

Assets and derived artifacts have content SHA-256 identities. A repeated byte-identical frame shares its artifact identity while `occurrences` preserve every source URI, parent asset and source time. Observations use the `filmmap.observation` envelope with a stable ID, class, kind, producer/version, structured value, evidence artifact references, status, confidence and source time in integer milliseconds (point or half-open interval). `filmmap synthesize` creates a canonical revisioned JSON index, scene segments derived from interval scene descriptions, and explicit overlap conflict issues. Location candidates remain distinct from embedded GPS facts and should preserve unknowns, alternatives, and direct/contextual evidence. Queries support text filtering, asset/kind filters and millisecond bounds; they are not semantic search.

## Agent extensibility

The bundled skill describes when and how to use FilmMap. The `tools/` manifest provides optional scripts. Agents choose them based on the active profile and task; FilmMap does not install packages, invoke network AI, mutate source media, or silently choose an external provider.
