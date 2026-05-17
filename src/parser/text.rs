use super::{ContentBlock, ParsedArticle, SourceFormat};

pub fn parse(raw: &str, default_title: String) -> ParsedArticle {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut title = default_title.clone();
    let mut title_from_heading = false;

    for para in raw.split("\n\n") {
        let stripped = para.trim();
        if stripped.is_empty() {
            continue;
        }

        if let Some(rest) = stripped.strip_prefix('#') {
            let mut level: u8 = 1;
            let mut chars = rest.chars();
            for ch in chars.by_ref() {
                if ch == '#' {
                    level += 1;
                } else {
                    break;
                }
            }
            let heading_text = stripped.trim_start_matches('#').trim().to_string();
            if !heading_text.is_empty() {
                if level == 1 && !title_from_heading {
                    title = heading_text.clone();
                    title_from_heading = true;
                }
                blocks.push(ContentBlock::heading(heading_text, level));
                continue;
            }
        }

        blocks.push(ContentBlock::text(stripped));
    }

    ParsedArticle { title, blocks, source_format: SourceFormat::Text }
}
