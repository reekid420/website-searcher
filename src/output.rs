use colored_json::ToColoredJson;
use serde_json::json;

use crate::models::SearchResult;

pub fn print_pretty_json(results: &[SearchResult]) {
    let value = json!({
        "results": results,
        "count": results.len(),
    });
    match serde_json::to_string_pretty(&value) {
        Ok(s) => match s.to_colored_json_auto() {
            Ok(cs) => println!("{}", cs),
            Err(_) => println!("{}", s),
        },
        Err(e) => eprintln!("failed to serialize results: {}", e),
    }
}

pub fn print_table_grouped(results: &[SearchResult]) {
    if results.is_empty() {
        println!("No results.");
        return;
    }
    let mut current_site: Option<&str> = None;
    for r in results {
        if current_site.is_none_or(|s| !s.eq_ignore_ascii_case(&r.site)) {
            if current_site.is_some() {
                println!();
            }
            println!("{}:", r.site);
            current_site = Some(&r.site);
        }
        let url = r.url.replace("/./", "/");
        println!("  - {} ({})", r.title, url);
    }
}
