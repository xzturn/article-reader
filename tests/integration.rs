use article_reader::config::alias_to_voice;
use article_reader::parser::{BlockKind, parse_file};
use article_reader::preprocess::{preprocess, preprocess_ssml};
use article_reader::splitter::{sanitize_filename, split_by_sections};
use article_reader::tts::{parse_speed, resolve_voice};
use std::path::PathBuf;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_samples")
}

#[test]
fn parses_simple_txt() {
    let article = parse_file(&samples_dir().join("simple.txt")).unwrap();
    assert!(!article.blocks.is_empty());
    assert!(article.blocks.iter().all(|b| b.kind == BlockKind::Text));
    let text = preprocess(&article);
    assert!(text.contains("春天"));
    assert!(!text.contains("\n\n\n"));
}

#[test]
fn parses_mixed_markdown_and_skips_special_blocks() {
    let article = parse_file(&samples_dir().join("mixed.md")).unwrap();
    assert_eq!(article.title, "Python 入门指南");

    let has_code = article.blocks.iter().any(|b| b.kind == BlockKind::Code);
    let has_table = article.blocks.iter().any(|b| b.kind == BlockKind::Table);
    assert!(has_code, "应识别到代码块");
    assert!(has_table, "应识别到表格");

    let text = preprocess(&article);
    assert!(text.contains("（这里有一段代码，已跳过）"));
    assert!(text.contains("（这里有一个表格，已跳过）"));
    assert!(!text.contains("```"));
    assert!(!text.contains("**"));
    assert!(text.contains("Python 官网"));
    assert!(!text.contains("(https://python.org)"));
}

#[test]
fn parses_html_strips_nav_and_script() {
    let article = parse_file(&samples_dir().join("article.html")).unwrap();
    assert_eq!(article.title, "人工智能的未来");

    let text = preprocess(&article);
    assert!(text.contains("人工智能"));
    assert!(!text.contains("console.log"));
    assert!(!text.contains("首页"), "导航项应被剥离");
    assert!(text.contains("（这里有一段代码，已跳过）"));
    assert!(text.contains("（这里有一个表格，已跳过）"));
}

#[test]
fn splits_long_article_by_sections() {
    let article = parse_file(&samples_dir().join("long_article.md")).unwrap();
    let sections = split_by_sections(&article);
    assert!(sections.len() >= 2, "长文章应拆出多个章节");
    for (title, text) in &sections {
        assert!(!title.is_empty());
        assert!(!text.trim().is_empty());
    }
}

#[test]
fn sanitize_filename_keeps_chinese() {
    let s = sanitize_filename("唐诗：李白与杜甫!", 20);
    assert!(s.contains("唐诗"));
    assert!(!s.contains("!"));
    assert!(!s.contains("："));

    assert_eq!(sanitize_filename("!@#$%", 10), "untitled");
}

#[test]
fn voice_alias_resolution() {
    assert_eq!(alias_to_voice("xiaoxiao").unwrap(), "zh-CN-XiaoxiaoNeural");
    assert_eq!(resolve_voice("xiaoxiao"), "zh-CN-XiaoxiaoNeural");
    assert_eq!(resolve_voice("yunxi"), "zh-CN-YunxiNeural");
    assert_eq!(resolve_voice("zh-CN-Custom"), "zh-CN-Custom");
}

#[test]
fn parses_speed_strings() {
    assert_eq!(parse_speed("+0%").unwrap(), 0);
    assert_eq!(parse_speed("+20%").unwrap(), 20);
    assert_eq!(parse_speed("-10%").unwrap(), -10);
    assert_eq!(parse_speed("5").unwrap(), 5);
    assert!(parse_speed("foo").is_err());
}

#[test]
fn ssml_wraps_blocks_with_breaks() {
    let article = parse_file(&samples_dir().join("mixed.md")).unwrap();
    let ssml = preprocess_ssml(&article);
    assert!(ssml.starts_with("<speak version=\"1.0\""));
    assert!(ssml.ends_with("</speak>"));
    assert!(ssml.contains("<break time=\"800ms\"/>"));
}
