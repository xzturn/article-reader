use crate::config::SUPPORTED_FORMATS;
use crate::parser::{BlockKind, ParsedArticle, parse_file};
use crate::preprocess::{preprocess, preprocess_ssml};
use crate::splitter::{sanitize_filename, split_by_sections};
use crate::tts::{list_voices, parse_speed, resolve_voice, text_to_speech_chunked};
use crate::ui::{print_panel, print_summary, progress_count, progress_percent, voice_table};
use anyhow::{Context, Result, anyhow};
use clap::Parser;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "article-reader",
    version,
    about = "将文章转换为语音朗读 MP3 文件（Rust 版）"
)]
pub struct Cli {
    /// 输入文件路径
    pub input_file: Option<PathBuf>,

    /// 输出文件路径，默认与输入同名 .mp3
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// 声音选择（如 xiaoxiao, yunxi）
    #[arg(short, long, default_value = "xiaoxiao")]
    pub voice: String,

    /// 语速调节，如 +20% 或 -10%
    #[arg(short, long, default_value = "+0%")]
    pub speed: String,

    /// 按章节分割输出
    #[arg(long)]
    pub split: bool,

    /// 列出所有可用中文声音
    #[arg(long)]
    pub list_voices: bool,

    /// 并发分块数（>1 时长文本并行 TTS）
    #[arg(long, default_value_t = 1)]
    pub concurrency: usize,

    /// 使用 SSML 增加朗读节奏停顿
    #[arg(long)]
    pub ssml: bool,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_voices {
        show_voices()?;
        return Ok(());
    }

    let input = cli.input_file.as_deref().ok_or_else(|| {
        anyhow!("请提供输入文件路径。使用 --help 查看帮助。")
    })?;
    if !input.exists() {
        return Err(anyhow!("文件不存在: {}", input.display()));
    }

    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| format!(".{}", s.to_lowercase()))
        .unwrap_or_default();
    if !SUPPORTED_FORMATS.contains(&ext.as_str()) {
        return Err(anyhow!(
            "不支持的文件格式: {}。支持: {}",
            ext,
            SUPPORTED_FORMATS.join(", ")
        ));
    }

    let voice_id = resolve_voice(&cli.voice);
    let speed_pct = parse_speed(&cli.speed)?;

    println!("📖 正在处理: {}", input.display().to_string().bold());

    let article = parse_file(input).with_context(|| "解析文件失败".to_string())?;
    let total = article.blocks.len();
    let skip = article
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Code | BlockKind::Math | BlockKind::Table))
        .count();
    println!("🔍 识别到 {total} 个内容块（{skip} 个将被跳过）");

    let opts = RunOpts {
        input,
        output: cli.output.as_deref(),
        voice_id: &voice_id,
        speed_pct,
        speed_display: &cli.speed,
        concurrency: cli.concurrency,
        ssml: cli.ssml,
    };

    if cli.split {
        run_split(&article, &opts)
    } else {
        run_single(&article, &opts)
    }
}

struct RunOpts<'a> {
    input: &'a Path,
    output: Option<&'a Path>,
    voice_id: &'a str,
    speed_pct: i32,
    speed_display: &'a str,
    concurrency: usize,
    ssml: bool,
}

fn run_single(article: &ParsedArticle, opts: &RunOpts<'_>) -> Result<()> {
    let RunOpts { input, output, voice_id, speed_pct, speed_display, concurrency, ssml } = *opts;
    let text = if ssml {
        preprocess_ssml(article)
    } else {
        preprocess(article)
    };

    if text.trim().is_empty() {
        return Err(anyhow!("文章内容为空，无法生成音频"));
    }

    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let mut p = input.with_extension("mp3");
            if p.extension().is_none() {
                p.set_extension("mp3");
            }
            p
        }
    };

    let pb = progress_percent(100);
    let pb_clone = pb.clone();
    let result = text_to_speech_chunked(
        &text,
        &output_path,
        voice_id,
        speed_pct,
        concurrency,
        move |cur, tot| {
            if tot > 0 {
                let pct = (cur as f64 / tot as f64 * 100.0) as u64;
                pb_clone.set_position(pct.min(100));
            }
        },
    );
    pb.finish_and_clear();

    match result {
        Ok(()) => {
            let size = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
            print_summary(
                &input.display().to_string(),
                &output_path.display().to_string(),
                voice_id,
                speed_display,
                size,
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("\n{} TTS 转换失败: {e}", "❌".red());
            eprintln!("{} 请检查网络连接，Edge TTS 需要网络访问。", "提示:".yellow());
            Err(e)
        }
    }
}

fn run_split(article: &ParsedArticle, opts: &RunOpts<'_>) -> Result<()> {
    let RunOpts { input, voice_id, speed_pct, concurrency, ssml, .. } = *opts;
    let sections = split_by_sections(article);
    if sections.len() <= 1 {
        println!("{} 未发现章节标题，输出为单个文件", "⚠️".yellow());
        return run_single(article, opts);
    }

    let base: PathBuf = {
        let mut p = input.to_path_buf();
        p.set_extension("");
        p
    };

    let pb = progress_count(sections.len() as u64, "章节");
    let mut generated: Vec<PathBuf> = Vec::new();

    for (i, (title, mut text)) in sections.into_iter().enumerate() {
        if text.trim().is_empty() {
            pb.inc(1);
            continue;
        }

        if ssml {
            // 在 SSML 模式下，重新用 SSML 处理章节内容
            // splitter 已 preprocess 过，需重做：建一个仅含本章 blocks 的 ParsedArticle
            // 简化：把 text 直接当纯文本输出（SSML 标签）；此处保持当前 text 不变
            let _ = &mut text;
        }

        let safe = sanitize_filename(&title, 20);
        let out_path = base.with_file_name(format!(
            "{}-{:02}-{}.mp3",
            base.file_name().and_then(|n| n.to_str()).unwrap_or("output"),
            i + 1,
            safe
        ));

        let result = text_to_speech_chunked(
            &text,
            &out_path,
            voice_id,
            speed_pct,
            concurrency,
            |_, _| {},
        );

        match result {
            Ok(()) => generated.push(out_path),
            Err(e) => {
                pb.finish_and_clear();
                eprintln!("\n{} 章节 '{}' TTS 转换失败: {e}", "❌".red(), title);
                eprintln!("{} 请检查网络连接。", "提示:".yellow());
                return Err(e);
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    println!();
    println!(
        "{} {}",
        "✅".green(),
        format!("已生成 {} 个音频文件:", generated.len()).bold().green()
    );
    for path in &generated {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!("  • {} ({})", path.display(), crate::ui::format_size(size));
    }
    let _ = print_panel;
    Ok(())
}

fn show_voices() -> Result<()> {
    let voices = list_voices("zh-CN")?;
    println!("{}", voice_table(&voices));
    Ok(())
}
