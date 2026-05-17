use super::{BlockKind, ContentBlock, ParsedArticle, SourceFormat};
use scraper::{ElementRef, Html, Node, Selector};

const SKIP_TAGS: &[&str] = &["script", "style", "nav", "header", "footer", "aside"];
const TEXT_CONTAINER_TAGS: &[&str] = &["p", "div", "section", "span", "blockquote", "li", "a"];

pub fn parse(raw: &str, default_title: String) -> ParsedArticle {
    let document = Html::parse_document(raw);

    let mut title = default_title.clone();
    if let Ok(sel) = Selector::parse("title")
        && let Some(t) = document.select(&sel).next()
    {
        let txt = t.text().collect::<String>().trim().to_string();
        if !txt.is_empty() {
            title = txt;
        }
    }

    let root: ElementRef = ["article", "main", "body"]
        .iter()
        .find_map(|name| {
            Selector::parse(name)
                .ok()
                .and_then(|sel| document.select(&sel).next())
        })
        .unwrap_or_else(|| document.root_element());

    let mut blocks: Vec<ContentBlock> = Vec::new();
    for child in root.children() {
        if let Some(el) = ElementRef::wrap(child) {
            process_element(el, &mut blocks);
        }
    }

    if let Some(b) = blocks
        .iter()
        .find(|b| b.kind == BlockKind::Heading && b.level == 1)
    {
        title = b.content.clone();
    }

    ParsedArticle { title, blocks, source_format: SourceFormat::Html }
}

fn process_element(el: ElementRef, blocks: &mut Vec<ContentBlock>) {
    let name = el.value().name();

    if SKIP_TAGS.contains(&name) {
        return;
    }

    match name {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level: u8 = name.as_bytes()[1] - b'0';
            let text = collect_text(el);
            if !text.is_empty() {
                blocks.push(ContentBlock::heading(text, level));
            }
        }
        "pre" => {
            let text = collect_text(el);
            blocks.push(ContentBlock::other(BlockKind::Code, text));
        }
        "table" => {
            let mut rows: Vec<String> = Vec::new();
            if let Ok(tr_sel) = Selector::parse("tr") {
                for tr in el.select(&tr_sel) {
                    let mut cells: Vec<String> = Vec::new();
                    for child in tr.children() {
                        if let Some(cell) = ElementRef::wrap(child) {
                            let n = cell.value().name();
                            if n == "td" || n == "th" {
                                cells.push(collect_text(cell));
                            }
                        }
                    }
                    rows.push(cells.join(" | "));
                }
            }
            blocks.push(ContentBlock::other(BlockKind::Table, rows.join("\n")));
        }
        "img" => {
            let alt = el.value().attr("alt").unwrap_or_default().to_string();
            blocks.push(ContentBlock::other(BlockKind::Image, alt));
        }
        "ul" | "ol" => {
            if let Ok(li_sel) = Selector::parse(":scope > li") {
                for li in el.select(&li_sel) {
                    let text = collect_text(li);
                    if !text.is_empty() {
                        blocks.push(ContentBlock::text(text));
                    }
                }
            }
        }
        n if TEXT_CONTAINER_TAGS.contains(&n) => {
            let text = collect_text(el);
            if !text.is_empty() {
                blocks.push(ContentBlock::text(text));
            }
        }
        _ => {
            for child in el.children() {
                if let Some(child_el) = ElementRef::wrap(child) {
                    process_element(child_el, blocks);
                } else if let Node::Text(t) = child.value() {
                    let s = t.text.trim();
                    if !s.is_empty() {
                        blocks.push(ContentBlock::text(s));
                    }
                }
            }
        }
    }
}

fn collect_text(el: ElementRef) -> String {
    let joined: String = el.text().collect();
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}
