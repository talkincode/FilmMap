# FilmMap

FilmMap defines a reusable, Agent-native workflow for turning media files into evidence-backed, searchable editing indexes. The Rust `filmmap` CLI owns stable IDs, capability profiles, requirement planning, index validation and queries. It does not implement media understanding: the host Agent or external tools such as FFmpeg, OCR, ASR and vision providers supply observations under declared capabilities.

## Quick start

```sh
filmmap init ./filmmap-project
filmmap profile detect --output ./filmmap-project/profile.json
filmmap capability list
filmmap-scan.sh /path/to/media-directory ./filmmap-project
filmmap-scan.sh ./media ./filmmap-project --frames 2
filmmap observe add ./filmmap-project/index.jsonl ASSET_ID \
  --kind scene_description --class interpretation \
  --start-ms 12000 --end-ms 18000 \
  --value-json '{"text":"person walking beside a lake"}' \
  --evidence-ref FRAME_ARTIFACT_ID
filmmap synthesize ./filmmap-project/index.jsonl --out ./filmmap-project/index.json
filmmap validate ./filmmap-project/index.json
filmmap query ./filmmap-project/index.json "lake" --kind scene_description --from-ms 12000 --to-ms 18000
```

See the [Chinese guide](README_CN.md), [mdBook](https://talkincode.github.io/FilmMap/) and [capability coverage matrix](docs/roadmap.md#验收矩阵业务能力覆盖矩阵). Level 0 defines contracts and executable workflow; it does not bundle AI providers or alter source media.

## Install

**Homebrew:** `brew install talkincode/tap/filmmap`  
**APT:** download the matching `.deb` from [GitHub Releases](https://github.com/talkincode/FilmMap/releases), then `sudo apt install ./filmmap_*_amd64.deb` (or arm64).  
**curl:** `curl -fsSL https://raw.githubusercontent.com/talkincode/FilmMap/main/install.sh | sh`

Release CI publishes Linux amd64/arm64, macOS arm64/x86_64 archives, Linux `.deb` packages and checksums. Homebrew formula publishing requires repository secret `HOMEBREW_TAP_TOKEN` with write access to `talkincode/homebrew-tap`.

## Editing workflow

`filmmap profile detect` records locally discoverable executables. Vision, OCR, transcription and place inference remain `unverified` until the surrounding Agent/provider declares them. Use `filmmap plan` against a requirement before analysis. Observations support structured values, fact/measurement/interpretation class, acceptance status, millisecond points or half-open intervals, producer version and references to registered evidence artifacts. `filmmap synthesize` produces a deterministic canonical index with scene segments and conflict issues; queries remain text filtering rather than semantic retrieval. Distinguish embedded GPS from visual place inference. Consult [the dedicated FilmMap skill](skills/filmmap/SKILL.md) and install it with `filmmap install skill`.

The scripts in `tools/` are optional helpers, not hidden dependencies. Package installers place them alongside the CLI; `filmmap install tools` installs them to a custom folder. FFmpeg/ffprobe are external prerequisites. `filmmap-scan.sh <media-root> <workspace> [--frames] [interval]` repeats directory registration and metadata probing, with optional sampled frames.

## Development and quality

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build
tests/e2e/cli.sh target/debug/filmmap
bash tests/e2e/scan.sh target/debug/filmmap tools
mdbook build
```

Every top-level user capability needs a happy-path E2E and the [roadmap matrix](docs/roadmap.md#验收矩阵业务能力覆盖矩阵) must identify risk, failures and recovery. E2E verifies real CLI behavior; avoid tests that only inflate coverage.

## License

MIT. See [LICENSE](LICENSE).
