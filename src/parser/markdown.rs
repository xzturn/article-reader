use super::{BlockKind, ContentBlock, ParsedArticle, SourceFormat};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub fn parse(raw: &str, default_title: String) -> ParsedArticle {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut blocks: Vec<ContentBlock> = Vec::new();

    let mut buf = String::new();
    let mut current: Option<BlockKind> = None;
    let mut current_level: u8 = 0;
    let mut in_link = 0u32;
    let mut in_image = 0u32;
    let mut image_alt: Option<String> = None;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut in_table = false;
    let mut in_table_cell = false;

    let flush_text = |buf: &mut String, current: &mut Option<BlockKind>, level: u8, blocks: &mut Vec<ContentBlock>| {
        if let Some(kind) = current.take() {
            let text = buf.trim().to_string();
            buf.clear();
            if !text.is_empty() {
                match kind {
                    BlockKind::Heading => blocks.push(ContentBlock::heading(text, level)),
                    BlockKind::Text => blocks.push(ContentBlock::text(text)),
                    other => blocks.push(ContentBlock::other(other, text)),
                }
            }
        }
    };

    for event in Parser::new_ext(raw, opts) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
                current = Some(BlockKind::Heading);
                current_level = heading_level_to_u8(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Start(Tag::Paragraph) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
                current = Some(BlockKind::Text);
            }
            Event::End(TagEnd::Paragraph) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
                current = Some(BlockKind::Code);
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Start(Tag::Table(_)) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
                in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                let text = table_rows
                    .iter()
                    .map(|row| row.join(" | "))
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    blocks.push(ContentBlock::other(BlockKind::Table, text));
                }
                table_rows.clear();
            }
            Event::Start(Tag::TableRow) | Event::Start(Tag::TableHead) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                table_rows.push(std::mem::take(&mut current_row));
            }
            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                buf.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_table_cell = false;
                current_row.push(buf.trim().to_string());
                buf.clear();
            }
            Event::Start(Tag::Link { .. }) => {
                in_link += 1;
            }
            Event::End(TagEnd::Link) => {
                in_link = in_link.saturating_sub(1);
            }
            Event::Start(Tag::Image { .. }) => {
                in_image += 1;
                image_alt = Some(String::new());
            }
            Event::End(TagEnd::Image) => {
                in_image = in_image.saturating_sub(1);
                if let Some(alt) = image_alt.take()
                    && !alt.is_empty()
                    && !(current.is_some() || in_table)
                {
                    blocks.push(ContentBlock::other(BlockKind::Image, alt));
                }
            }
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {}
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {}
            Event::Start(Tag::BlockQuote(_)) | Event::End(TagEnd::BlockQuote(_)) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Start(Tag::List(_)) | Event::End(TagEnd::List(_)) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Start(Tag::Item) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
                current = Some(BlockKind::Text);
            }
            Event::End(TagEnd::Item) => {
                flush_text(&mut buf, &mut current, current_level, &mut blocks);
            }
            Event::Text(s) | Event::Code(s) => {
                if in_image > 0 {
                    if let Some(alt) = image_alt.as_mut() {
                        alt.push_str(&s);
                    }
                    if current.is_some() || in_table_cell {
                        buf.push_str(&s);
                    }
                } else if in_table_cell {
                    buf.push_str(&s);
                } else if current.is_some() {
                    let _ = in_link;
                    buf.push_str(&s);
                }
            }
            Event::SoftBreak | Event::HardBreak if current.is_some() => {
                buf.push('\n');
            }
            Event::Rule => {}
            _ => {}
        }
    }

    flush_text(&mut buf, &mut current, current_level, &mut blocks);

    let title = blocks
        .iter()
        .find(|b| b.kind == BlockKind::Heading && b.level == 1)
        .map(|b| b.content.clone())
        .unwrap_or(default_title);

    ParsedArticle { title, blocks, source_format: SourceFormat::Markdown }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
