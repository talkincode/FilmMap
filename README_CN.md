# FilmMap

FilmMap 为剪辑素材建立可复用、可检索、带证据来源的索引协议和 Agent 工作流。Rust CLI 负责稳定素材 ID、能力 profile、需求规划、索引校验与查询；画面理解、OCR、ASR、地理推断由宿主 Agent 或外部工具执行，FilmMap 不内置这些服务。

## 快速开始

```sh
filmmap init ./filmmap-project
filmmap profile detect --output ./filmmap-project/profile.json
filmmap capability list
filmmap artifact add ./filmmap-project/index.jsonl /素材/视频.mp4
filmmap validate ./filmmap-project/index.jsonl
filmmap query ./filmmap-project/index.jsonl "山景"
```

项目目标与验收边界见[开发路线图](docs/roadmap.md)；完整工作流见 [mdBook](https://talkincode.github.io/FilmMap/) 和[专用技能](skills/filmmap/SKILL.md)。

## 安装

- Homebrew：`brew install talkincode/tap/filmmap`
- APT：从 [GitHub Releases](https://github.com/talkincode/FilmMap/releases) 下载对应 `.deb`，执行 `sudo apt install ./filmmap_*_amd64.deb`（ARM64 替换架构）。
- curl：`curl -fsSL https://raw.githubusercontent.com/talkincode/FilmMap/main/install.sh | sh`

安装包会把 `filmmap-scan.sh` 等辅助脚本放在与 `filmmap` 相同的命令目录中，可直接调用。

Release CI 发布 Linux amd64/arm64、macOS arm64/x86_64、Linux deb 包及校验和。自动更新 Homebrew 需要仓库 secret `HOMEBREW_TAP_TOKEN`。

## 素材分析工作流

`filmmap profile detect` 探测本机可用程序。视觉理解、OCR、转录、地点推断在 profile 中保持 `unverified`，由当前 Agent/provider 明确声明后再执行。先用 requirement 和 profile 运行 `filmmap plan`，只执行已具备的能力。索引要保留来源、时间、置信度和证据：EXIF GPS 与画面推断地点必须区分，冲突保留，未知不猜。辅助脚本位于 `tools/`，通过 `filmmap install tools` 安装。批量整理可运行 `filmmap-scan.sh <素材目录> <工作区> [--frames] [间隔秒数]` 登记视频并保存 ffprobe 证据；按需加 `--frames` 抽帧。FFmpeg/ffprobe 需外部提供。

## 开发与验收

```sh
cargo fmt --check && cargo clippy -- -D warnings && cargo test
cargo build && tests/e2e/cli.sh target/debug/filmmap
bash tests/e2e/scan.sh target/debug/filmmap tools
mdbook build
```

新增一级能力必须新增真实 CLI E2E 并登记到[路线图验收矩阵](docs/roadmap.md#验收矩阵业务能力覆盖矩阵)。优先验证用户完整操作路径，不做只为覆盖率服务的测试。
