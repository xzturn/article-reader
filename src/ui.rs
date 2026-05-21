use crate::config::{VOICE_ALIASES, voice_to_alias};
use comfy_table::{Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use indicatif::{ProgressBar, ProgressStyle};
use msedge_tts::voice::Voice;
use owo_colors::OwoColorize;
use std::time::Duration;

pub fn progress_percent(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} {msg} [{bar:40.cyan/blue}] {percent:>3}% {elapsed}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    pb.set_message("转换中...");
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

pub fn progress_count(total: u64, unit: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            &format!("{{spinner:.cyan}} {{msg}} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} {} {{elapsed}}", unit),
        )
        .unwrap()
        .progress_chars("##-"),
    );
    pb.set_message("转换中...");
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

pub fn voice_table(voices: &[Voice]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("别名").fg(Color::Cyan),
            Cell::new("完整名称").fg(Color::Green),
            Cell::new("性别"),
        ]);

    let mut alias_rows: Vec<(String, String, String)> = Vec::new();
    let mut other_rows: Vec<(String, String, String)> = Vec::new();

    for v in voices {
        let short = v.short_name.as_deref().unwrap_or(&v.name);
        let alias = voice_to_alias(short).unwrap_or("-").to_string();
        let display = short.replace("zh-CN-", "").replace("Neural", "");
        let gender = match v.gender.as_deref() {
            Some("Female") => "女",
            _ => "男",
        }
        .to_string();
        if alias != "-" {
            alias_rows.push((alias, display, gender));
        } else {
            other_rows.push((alias, display, gender));
        }
    }

    alias_rows.sort_by(|a, b| {
        let order = |k: &str| {
            VOICE_ALIASES
                .iter()
                .position(|(n, _)| *n == k)
                .unwrap_or(usize::MAX)
        };
        order(&a.0).cmp(&order(&b.0))
    });

    for (a, n, g) in alias_rows.into_iter().chain(other_rows) {
        table.add_row(vec![a, n, g]);
    }

    table
}

pub fn print_summary(input: &str, output: &str, voice_id: &str, speed: &str, size_bytes: u64) {
    let alias = voice_to_alias(voice_id).unwrap_or(voice_id);
    let display = voice_id.replace("zh-CN-", "").replace("Neural", "");
    let lines = [
        format!("源文件:   {input}"),
        format!("输出文件: {output}"),
        format!("声音:     {alias} ({display})"),
        format!("语速:     {speed}"),
        format!("文件大小: {}", format_size(size_bytes)),
    ];
    print_panel("✅ 转换完成", &lines);
}

pub fn print_panel(title: &str, lines: &[String]) {
    let width = lines
        .iter()
        .map(|l| display_width(l))
        .chain(std::iter::once(display_width(title) + 4))
        .max()
        .unwrap_or(40)
        .max(40);

    let title_padding_total = width.saturating_sub(display_width(title) + 2);
    let left = title_padding_total / 2;
    let right = title_padding_total - left;
    println!(
        "{}",
        format!(
            "╭{} {} {}╮",
            "─".repeat(left),
            title,
            "─".repeat(right)
        )
        .green()
    );
    for line in lines {
        let pad = width.saturating_sub(display_width(line));
        println!("{} {}{} {}", "│".green(), line, " ".repeat(pad), "│".green());
    }
    println!("{}", format!("╰{}╯", "─".repeat(width + 2)).green());
}

pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    }
}

fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}
