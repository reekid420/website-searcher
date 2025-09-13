use colored_json::ToColoredJson;
use serde_json::json;

use crate::models::SearchResult;
use std::collections::BTreeMap;
use tabled::{Table, Tabled, settings::Style};
use terminal_size::{Width as TWidth, terminal_size};
use textwrap::fill as tw_fill;

#[allow(dead_code)]
pub fn calc_title_wrap_columns() -> usize {
    let term_cols = match terminal_size().map(|(w, _)| w) {
        Some(TWidth(n)) if n > 20 => n as usize,
        _ => 100usize,
    };
    term_cols.saturating_sub(40).max(20)
}

pub fn print_pretty_json(results: &[SearchResult]) {
    let value = json!({
        "results": results,
        "count": results.len(),
    });
    match serde_json::to_string_pretty(&value) {
        Ok(s) => match s.to_colored_json_auto() {
            Ok(cs) => println!("{cs}"),
            Err(_) => println!("{s}"),
        },
        Err(e) => eprintln!("failed to serialize results: {e}"),
    }
}

pub fn print_table_grouped(results: &[SearchResult]) {
    if results.is_empty() {
        println!("No results.");
        return;
    }
    // Deterministically group rows by site (alphabetical) so no site is dropped
    let mut grouped: BTreeMap<&str, Vec<DisplayRow>> = BTreeMap::new();
    for r in results {
        grouped
            .entry(&r.site)
            .or_default()
            .push(DisplayRow::from(r));
    }
    // Compute target wrap width
    let _term_cols = match terminal_size().map(|(w, _)| w) {
        Some(TWidth(n)) if n > 20 => n as usize,
        _ => 100usize,
    };
    let title_wrap = calc_title_wrap_columns();

    for (site, rows) in grouped.iter_mut() {
        if rows.is_empty() {
            continue;
        }
        // Wrap long titles to fit
        for r in rows.iter_mut() {
            if r.title.len() > title_wrap {
                r.title = tw_fill(&r.title, title_wrap);
            }
        }
        let mut table = Table::new(rows.clone());
        table.with(Style::rounded());
        println!("{site}:");
        if std::env::var("NO_TABLE").ok().as_deref() == Some("1") {
            for r in rows.iter() {
                println!("  - {} ({})", r.title, r.url);
            }
            println!();
        } else {
            println!("{table}\n");
        }
    }
}

#[derive(Clone, Tabled)]
struct DisplayRow {
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "URL")]
    url: String,
}

impl From<&SearchResult> for DisplayRow {
    fn from(r: &SearchResult) -> Self {
        Self {
            title: r.title.clone(),
            url: r.url.replace("/./", "/"),
        }
    }
}
