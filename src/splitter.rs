use crate::parser::{BlockKind, ContentBlock, ParsedArticle};
use crate::preprocess::preprocess;
use regex::Regex;
use std::sync::OnceLock;

pub fn split_by_sections(article: &ParsedArticle) -> Vec<(String, String)> {
    let mut sections: Vec<(String, Vec<ContentBlock>)> = Vec::new();
    let mut current_title = "开头".to_string();
    let mut current_blocks: Vec<ContentBlock> = Vec::new();

    for block in &article.blocks {
        if block.kind == BlockKind::Heading && block.level <= 2 {
            if !current_blocks.is_empty() {
                sections.push((current_title.clone(), std::mem::take(&mut current_blocks)));
            }
            current_title = block.content.trim().to_string();
            current_blocks.push(block.clone());
        } else {
            current_blocks.push(block.clone());
        }
    }

    if !current_blocks.is_empty() {
        sections.push((current_title, current_blocks));
    }

    if sections.is_empty() {
        return Vec::new();
    }

    if sections.len() == 1 && sections[0].0 == "开头" {
        let only = sections.into_iter().next().unwrap();
        let stub = ParsedArticle {
            title: article.title.clone(),
            blocks: only.1,
            source_format: article.source_format,
        };
        let text = preprocess(&stub);
        return vec![(article.title.clone(), text)];
    }

    let mut result: Vec<(String, String)> = Vec::new();
    for (title, blocks) in sections {
        let stub = ParsedArticle {
            title: title.clone(),
            blocks,
            source_format: article.source_format,
        };
        let text = preprocess(&stub);
        if !text.trim().is_empty() {
            result.push((title, text));
        }
    }
    result
}

pub fn sanitize_filename(name: &str, max_len: usize) -> String {
    let truncated: String = name.chars().take(max_len).collect();

    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[^\w\u{4e00}-\u{9fff}\-]").unwrap());
    let cleaned = re.replace_all(&truncated, "").to_string();

    if cleaned.is_empty() {
        "untitled".to_string()
    } else {
        cleaned
    }
}
