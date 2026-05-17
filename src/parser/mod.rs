use anyhow::{Context, Result, anyhow};
use std::path::Path;

mod html;
mod markdown;
mod text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Text,
    Heading,
    Code,
    Math,
    Table,
    Image,
}

#[derive(Debug, Clone)]
pub struct ContentBlock {
    pub kind: BlockKind,
    pub content: String,
    pub level: u8,
}

impl ContentBlock {
    pub fn text(content: impl Into<String>) -> Self {
        Self { kind: BlockKind::Text, content: content.into(), level: 0 }
    }
    pub fn heading(content: impl Into<String>, level: u8) -> Self {
        Self { kind: BlockKind::Heading, content: content.into(), level }
    }
    pub fn other(kind: BlockKind, content: impl Into<String>) -> Self {
        Self { kind, content: content.into(), level: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Markdown,
    Html,
    Text,
}

#[derive(Debug, Clone)]
pub struct ParsedArticle {
    pub title: String,
    pub blocks: Vec<ContentBlock>,
    pub source_format: SourceFormat,
}

pub fn parse_file(path: &Path) -> Result<ParsedArticle> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| format!(".{}", s.to_lowercase()))
        .unwrap_or_default();

    let raw = read_file_auto_encoding(path)
        .with_context(|| format!("读取文件失败: {}", path.display()))?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    match ext.as_str() {
        ".md" => Ok(markdown::parse(&raw, stem)),
        ".html" | ".htm" => Ok(html::parse(&raw, stem)),
        ".txt" => Ok(text::parse(&raw, stem)),
        other => Err(anyhow!("不支持的文件格式: {}。支持: .md, .txt, .html", other)),
    }
}

fn read_file_auto_encoding(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(String::new());
    }

    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(&bytes, true);
    let encoding = detector.guess(None, true);
    let (text, _, _) = encoding.decode(&bytes);
    Ok(text.into_owned())
}
