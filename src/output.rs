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


