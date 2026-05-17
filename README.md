# ArticleReader

将 Markdown / HTML / TXT 文章转换为语音 MP3 的命令行工具，底层使用 Microsoft Edge TTS。

Rust 实现，单文件二进制，无运行时依赖。

## 特性

- 支持 `.md` / `.txt` / `.html` 输入，自动检测编码
- 智能跳过代码块、数学公式、表格（朗读时给出简短提示）
- 长文章自动按段落 + 句子分块，逐块合成后拼接
- 按章节标题（`h1`/`h2`）切分输出多个 MP3 文件
- 多个中文声音可选（`xiaoxiao` / `yunxi` / `xiaohan` / `yunyang` 等）
- 语速可调（`+20%` / `-10%`）
- 可选并发分块加速
- 可选 SSML 模式增加朗读节奏停顿

## 安装

```bash
cargo build --release
# 二进制位于 target/release/article-reader
```

## 用法

```bash
# 基本用法：MD/HTML/TXT → MP3
article-reader article.md

# 指定输出路径与声音
article-reader article.md -o out.mp3 -v yunxi

# 调整语速
article-reader article.md -s +20%

# 长文章按章节切分
article-reader long_article.md --split

# 长文本并发分块（4 路并行）
article-reader long_article.md --concurrency 4

# 启用 SSML 增加朗读停顿
article-reader article.md --ssml

# 列出所有可用中文声音
article-reader --list-voices
```

完整选项：

| 选项 | 说明 | 默认 |
|---|---|---|
| `INPUT_FILE` | 输入文件路径 | — |
| `-o, --output PATH` | 输出 MP3 路径 | `<input>.mp3` |
| `-v, --voice ALIAS` | 声音别名或完整名 | `xiaoxiao` |
| `-s, --speed +N%` | 语速调节 | `+0%` |
| `--split` | 按 `h1`/`h2` 章节切分多文件 | 关 |
| `--list-voices` | 仅打印声音表 | — |
| `--concurrency N` | 并发分块数 | `1` |
| `--ssml` | 使用 SSML 朗读节奏 | 关 |

## 项目结构

```
src/
├── main.rs           入口
├── lib.rs            模块导出
├── cli.rs            clap CLI + 顶层流程
├── config.rs         常量与声音别名
├── parser/
│   ├── mod.rs        ContentBlock 类型 + 编码检测分派
│   ├── markdown.rs   pulldown-cmark 适配
│   ├── html.rs       scraper 适配
│   └── text.rs       纯文本分段
├── preprocess.rs     朗读文本与 SSML 生成
├── splitter.rs       章节切分与文件名清洗
├── tts.rs            msedge-tts + 分块/重试/并发
├── concat.rs         MP3 字节拼接
└── ui.rs             进度条/表格/摘要 Panel

tests/
└── integration.rs    端到端解析与切分断言

test_samples/         离线样本（MD/HTML/TXT + 参考 MP3）
```

## 关键依赖

| 用途 | crate |
|---|---|
| Edge TTS WebSocket 客户端 | `msedge-tts` |
| Markdown 解析 | `pulldown-cmark` |
| HTML 解析 | `scraper` (html5ever) |
| 编码检测 | `chardetng` + `encoding_rs` |
| CLI | `clap` (derive) |
| 终端 UI | `indicatif` + `comfy-table` + `owo-colors` |

## 关于 MP3 拼接

长文本被切分为多个 chunk 分别合成，最终通过**原始字节拼接**合成单个 MP3。

Edge TTS 输出固定格式 CBR MP3（`audio-24khz-48kbitrate-mono-mp3`），无 ID3 标签、无 VBR header，MP3 帧自同步，因此直接 `std::io::copy` 拼接得到的文件可被所有主流播放器（ffplay / VLC / macOS 预览 / 浏览器）正常解码。无需 ffmpeg。

## 测试

```bash
cargo test          # 8 个端到端测试，覆盖三种解析器、预处理、切分、声音别名
cargo clippy --all-targets -- -D warnings
```

集成测试只覆盖纯解析/处理逻辑，不打网络；TTS 链路通过手工跑 `test_samples/` 验证。

## 网络要求

Edge TTS 通过 WebSocket 连接微软服务（无需鉴权）。中国大陆部分网络可能间歇性 403，重试通常即可（已内置 3 次指数退避）。

## License

MIT
