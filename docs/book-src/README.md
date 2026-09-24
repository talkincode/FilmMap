# FilmMap 工作流

FilmMap 为 Agent 提供素材索引协议，不替代视觉模型或剪辑软件。一次可靠的整理流程是：确认素材范围 → 探测工具并声明 profile → 按任务规划能力 → 抽取有时间戳的证据 → 写入观察结果 → 校验索引 → 按内容和时间查询并交给剪辑。

```sh
filmmap init ./filmmap-project
filmmap profile detect --output ./filmmap-project/profile.json
filmmap artifact add ./filmmap-project/index.jsonl /media/clip.mp4
```

`filmmap install skill` 安装 Agent 工作指引；`filmmap install tools` 安装可选 FFmpeg 辅助脚本。目录素材可通过 `filmmap-scan.sh <素材目录> <工作区> [--frames] [间隔秒数]` 批量登记和探测。缺少模型能力时先标为 unverified 或 unavailable，不把没有抽样到的内容写成事实。
