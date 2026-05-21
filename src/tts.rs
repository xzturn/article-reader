use crate::concat::merge_mp3;
use crate::config::{DEFAULT_VOICE, MAX_CHUNK_SIZE, alias_to_voice};
use anyhow::{Context, Result, anyhow};
use msedge_tts::tts::SpeechConfig;
use msedge_tts::tts::client::connect;
use msedge_tts::voice::{Voice, get_voices_list};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

pub fn resolve_voice(voice: &str) -> String {
    alias_to_voice(voice).unwrap_or(voice).to_string()
}

pub fn list_voices(language: &str) -> Result<Vec<Voice>> {
    let voices = get_voices_list().map_err(|e| anyhow!("获取声音列表失败: {e}"))?;
    Ok(voices
        .into_iter()
        .filter(|v| v.locale.as_deref().map(|l| l.starts_with(language)).unwrap_or(false))
        .collect())
}

pub fn parse_speed(speed: &str) -> Result<i32> {
    let s = speed.trim().trim_end_matches('%').trim();
    let s = if let Some(stripped) = s.strip_prefix('+') { stripped } else { s };
    s.parse::<i32>()
        .with_context(|| format!("无法解析语速参数: {speed}"))
}

fn make_config(voice: &str, speed_pct: i32) -> SpeechConfig {
    SpeechConfig {
        voice_name: voice.to_string(),
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: speed_pct,
        volume: 0,
    }
}

pub fn text_to_speech_chunked<F>(
    text: &str,
    output: &Path,
    voice: &str,
    speed_pct: i32,
    concurrency: usize,
    on_progress: F,
) -> Result<()>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }

    let voice = if voice.is_empty() { DEFAULT_VOICE } else { voice };
    let chunks = split_text(text, MAX_CHUNK_SIZE);
    let total = chunks.len();
    if total == 0 {
        return Ok(());
    }

    let config = make_config(voice, speed_pct);

    if total == 1 {
        on_progress(0, 1);
        let bytes = synthesize_with_retry(&chunks[0], &config, 3)?;
        fs::write(output, bytes).context("写入输出文件失败")?;
        on_progress(1, 1);
        return Ok(());
    }

    let tmp_dir = tempdir_random("article_reader_")?;
    let result = (|| -> Result<()> {
        let tmp_files: Vec<PathBuf> = (0..total)
            .map(|i| tmp_dir.join(format!("chunk_{i:04}.mp3")))
            .collect();

        if concurrency <= 1 {
            for (i, chunk) in chunks.iter().enumerate() {
                let bytes = synthesize_with_retry(chunk, &config, 3)?;
                fs::write(&tmp_files[i], bytes)?;
                on_progress(i + 1, total);
            }
        } else {
            run_parallel(&chunks, &tmp_files, &config, concurrency, &on_progress)?;
        }

        on_progress(total, total);
        merge_mp3(&tmp_files, output).context("合并 MP3 失败")?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

fn run_parallel<F>(
    chunks: &[String],
    tmp_files: &[PathBuf],
    config: &SpeechConfig,
    concurrency: usize,
    on_progress: &F,
) -> Result<()>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let total = chunks.len();
    let next_idx = Mutex::new(0usize);
    let done = AtomicUsize::new(0);
    let error = Mutex::new(None::<anyhow::Error>);

    thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..concurrency.min(total) {
            handles.push(scope.spawn(|| {
                loop {
                    let i = {
                        let mut g = next_idx.lock().unwrap();
                        if *g >= total || error.lock().unwrap().is_some() {
                            return;
                        }
                        let i = *g;
                        *g += 1;
                        i
                    };
                    match synthesize_with_retry(&chunks[i], config, 3) {
                        Ok(bytes) => {
                            if let Err(e) = fs::write(&tmp_files[i], bytes) {
                                *error.lock().unwrap() = Some(e.into());
                                return;
                            }
                            let cnt = done.fetch_add(1, Ordering::Relaxed) + 1;
                            on_progress(cnt, total);
                        }
                        Err(e) => {
                            *error.lock().unwrap() = Some(e);
                            return;
                        }
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
    });

    if let Some(e) = error.lock().unwrap().take() {
        return Err(e);
    }
    Ok(())
}

fn synthesize_with_retry(text: &str, config: &SpeechConfig, max_retries: u32) -> Result<Vec<u8>> {
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..max_retries {
        match try_synthesize(text, config) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < max_retries {
                    thread::sleep(Duration::from_secs(1u64 << attempt));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("TTS 失败")))
}

fn try_synthesize(text: &str, config: &SpeechConfig) -> Result<Vec<u8>> {
    let mut client = connect().map_err(|e| anyhow!("连接 Edge TTS 失败: {e}"))?;
    let audio = client
        .synthesize(text, config)
        .map_err(|e| anyhow!("合成失败: {e}"))?;
    Ok(audio.audio_bytes)
}

fn tempdir_random(prefix: &str) -> Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("{prefix}{pid}_{nanos}"));
    fs::create_dir_all(&path).with_context(|| format!("创建临时目录失败: {}", path.display()))?;
    Ok(path)
}

fn split_text(text: &str, chunk_size: usize) -> Vec<String> {
    if char_len(text) <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    for raw_para in text.split("\n\n") {
        let para = raw_para.trim();
        if para.is_empty() {
            continue;
        }

        let para_len = char_len(para);
        let current_len = char_len(&current);
        let sep_len = if current_len == 0 { 0 } else { 2 };

        if current_len + para_len + sep_len <= chunk_size {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
        } else {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            if para_len > chunk_size {
                chunks.extend(split_long_paragraph(para, chunk_size));
            } else {
                current.push_str(para);
            }
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn split_long_paragraph(text: &str, chunk_size: usize) -> Vec<String> {
    let sentence_enders: &[char] = &['。', '！', '？', '；', '.', '!', '?', ';'];

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut seg = String::new();

    for ch in text.chars() {
        seg.push(ch);
        if sentence_enders.contains(&ch) {
            if char_len(&current) + char_len(&seg) <= chunk_size {
                current.push_str(&seg);
            } else {
                if !current.is_empty() {
                    chunks.push(std::mem::take(&mut current));
                }
                if char_len(&seg) > chunk_size {
                    chunks.extend(hard_split(&seg, chunk_size));
                } else {
                    current.push_str(&seg);
                }
            }
            seg.clear();
        }
    }

    if !seg.is_empty() {
        if char_len(&current) + char_len(&seg) <= chunk_size {
            current.push_str(&seg);
        } else {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            if char_len(&seg) > chunk_size {
                chunks.extend(hard_split(&seg, chunk_size));
            } else {
                current.push_str(&seg);
            }
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn hard_split(text: &str, chunk_size: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|c| c.iter().collect::<String>())
        .collect()
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}
