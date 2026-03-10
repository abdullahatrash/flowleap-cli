use comfy_table::{Cell, Table};
use serde_json::Value;

/// Print raw JSON (for --output json)
pub fn print_json(value: &Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
}

/// Print a JSON array as a table (for --output table)
pub fn print_table(rows: &[Value], columns: &[(&str, &str)]) {
    let mut table = Table::new();
    table.set_header(columns.iter().map(|(_, header)| Cell::new(header)));

    for row in rows {
        let cells: Vec<Cell> = columns
            .iter()
            .map(|(key, _)| {
                let val = row.get(key).cloned().unwrap_or(Value::Null);
                match val {
                    Value::String(s) => Cell::new(truncate(&s, 50)),
                    Value::Null => Cell::new("-"),
                    other => Cell::new(other.to_string()),
                }
            })
            .collect();
        table.add_row(cells);
    }

    println!("{table}");
}

/// Print a value in human-readable format
pub fn print_value(format: &str, value: &Value, columns: &[(&str, &str)]) {
    match format {
        "json" => print_json(value),
        "table" => {
            if let Some(arr) = value.as_array() {
                print_table(arr, columns);
            } else {
                print_json(value);
            }
        }
        _ => {
            // Human-readable: pretty print with some structure
            if let Some(arr) = value.as_array() {
                if arr.is_empty() {
                    println!("No results found.");
                    return;
                }
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        println!("---");
                    }
                    print_object_human(item, columns);
                }
                println!("\n{} result(s)", arr.len());
            } else {
                print_object_human(value, columns);
            }
        }
    }
}

fn print_object_human(obj: &Value, columns: &[(&str, &str)]) {
    for (key, label) in columns {
        if let Some(val) = obj.get(key) {
            match val {
                Value::String(s) => println!("  {}: {}", label, s),
                Value::Null => {}
                other => println!("  {}: {}", label, other),
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
