use crate::parser::{BlockKind, ContentBlock, ParsedArticle};
use regex::Regex;
use std::sync::OnceLock;

const SKIP_CODE: &str = "（这里有一段代码，已跳过）";
const SKIP_MATH: &str = "（这里有一个数学公式，已跳过）";
const SKIP_TABLE: &str = "（这里有一个表格，已跳过）";

pub fn preprocess(article: &ParsedArticle) -> String {
    if article.blocks.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = article
        .blocks
        .iter()
        .filter_map(process_block)
        .collect();

    let joined = parts.join("\n\n");
    clean_text(&joined)
}

fn process_block(block: &ContentBlock) -> Option<String> {
    match block.kind {
        BlockKind::Text | BlockKind::Heading => {
            let t = block.content.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        }
        BlockKind::Code => Some(SKIP_CODE.to_string()),
        BlockKind::Math => Some(SKIP_MATH.to_string()),
        BlockKind::Table => Some(SKIP_TABLE.to_string()),
        BlockKind::Image => {
            let alt = block.content.trim();
            if alt.is_empty() {
                None
            } else {
                Some(format!("（图片：{alt}）"))
            }
        }
    }
}

fn clean_text(text: &str) -> String {
    let text = unescape_html(text);

    static RE_BQ: OnceLock<Regex> = OnceLock::new();
    static RE_LINK: OnceLock<Regex> = OnceLock::new();
    static RE_IMG: OnceLock<Regex> = OnceLock::new();
    static RE_BLANK: OnceLock<Regex> = OnceLock::new();

    let re_bq = RE_BQ.get_or_init(|| Regex::new(r"(?m)^>\s?").unwrap());
    let re_link = RE_LINK.get_or_init(|| Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap());
    let re_img = RE_IMG.get_or_init(|| Regex::new(r"!\[([^\]]*)\]\([^)]+\)").unwrap());
    let re_blank = RE_BLANK.get_or_init(|| Regex::new(r"\n{3,}").unwrap());

    let text = strip_markdown_marks(&text);
    let text = re_bq.replace_all(&text, "");
    let text = re_link.replace_all(&text, "$1");
    let text = re_img.replace_all(&text, "$1");
    let text = re_blank.replace_all(&text, "\n\n");

    text.trim().to_string()
}

fn strip_markdown_marks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '*' || c == '_' {
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let count = if i + 1 < chars.len() && chars[i + 1] == c { 2 } else { 1 };
            let next_idx = i + count;
            let next = chars.get(next_idx).copied().unwrap_or(' ');

            let opening = prev.is_whitespace() || is_punct(prev);
            let closing = next.is_whitespace() || is_punct(next);

            if opening && !next.is_whitespace() && next != c {
                i += count;
                continue;
            }
            if closing && !prev.is_whitespace() && prev != c {
                i += count;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_punct(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | '!' | '?' | ';' | ':' | '"' | '\''
            | '。' | '，' | '！' | '？' | '；' | '：'
            | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}'
    )
}

fn unescape_html(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

pub fn preprocess_ssml(article: &ParsedArticle) -> String {
    let header = r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="zh-CN">"#;
    let footer = "</speak>";

    if article.blocks.is_empty() {
        return format!("{header}{footer}");
    }

    let mut parts: Vec<String> = Vec::new();
    for block in &article.blocks {
        match block.kind {
            BlockKind::Heading => {
                parts.push(r#"<break time="800ms"/>"#.to_string());
                parts.push(escape_ssml(block.content.trim()));
                parts.push(r#"<break time="500ms"/>"#.to_string());
            }
            BlockKind::Text => {
                let t = block.content.trim();
                if !t.is_empty() {
                    parts.push(escape_ssml(t));
                    parts.push(r#"<break time="300ms"/>"#.to_string());
                }
            }
            BlockKind::Code => {
                parts.push(escape_ssml(SKIP_CODE));
                parts.push(r#"<break time="300ms"/>"#.to_string());
            }
            BlockKind::Math => {
                parts.push(escape_ssml(SKIP_MATH));
                parts.push(r#"<break time="300ms"/>"#.to_string());
            }
            BlockKind::Table => {
                parts.push(escape_ssml(SKIP_TABLE));
                parts.push(r#"<break time="300ms"/>"#.to_string());
            }
            BlockKind::Image => {
                let alt = block.content.trim();
                if !alt.is_empty() {
                    parts.push(escape_ssml(&format!("（图片：{alt}）")));
                }
            }
        }
    }

    format!("{header}\n{}\n{footer}", parts.join("\n"))
}

fn escape_ssml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
