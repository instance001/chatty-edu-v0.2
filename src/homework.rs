use serde::Deserialize;
use serde_json;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct CompletedHomework {
    pub student_id: String,
    pub student_name: String,
    pub homework_id: String,
    pub title: String,
    pub submitted_at: String,
    pub score: i32,
    pub out_of: i32,
}

/// Load all completed homework JSON files from:
///   <base_path>/homework/completed
pub fn load_completed_homework(base_path: &Path) -> io::Result<Vec<CompletedHomework>> {
    let completed_dir = base_path.join("homework").join("completed");
    let mut results = Vec::new();

    if !completed_dir.exists() {
        // Nothing completed yet – that's fine.
        return Ok(results);
    }

    for entry in fs::read_dir(&completed_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
            match fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<CompletedHomework>(&contents) {
                    Ok(h) => results.push(h),
                    Err(e) => {
                        eprintln!(
                            "[WARN] Skipping file {:?} – JSON parse error: {}",
                            path.file_name().unwrap_or_default(),
                            e
                        );
                    }
                },
                Err(e) => {
                    eprintln!(
                        "[WARN] Could not read file {:?}: {}",
                        path.file_name().unwrap_or_default(),
                        e
                    );
                }
            }
        }
    }

    Ok(results)
}

/// Print a nice table of completed homework entries.
pub fn print_homework_table(items: &[CompletedHomework]) {
    if items.is_empty() {
        println!("\nNo completed homework found yet.\n");
        return;
    }

    println!();
    println!(
        "{:<12} | {:<18} | {:<10} | {:<9} | {}",
        "Student", "Homework", "Score", "HW ID", "Submitted"
    );
    println!("{}", "-".repeat(80));

    for h in items {
        let score_str = format!("{}/{}", h.score, h.out_of);
        println!(
            "{:<12} | {:<18} | {:<10} | {:<9} | {}",
            truncate_for_table(&h.student_name, 12),
            truncate_for_table(&h.title, 18),
            score_str,
            truncate_for_table(&h.homework_id, 9),
            h.submitted_at,
        );
    }

    println!();
}

fn truncate_for_table(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut out = String::new();
        for (i, ch) in s.chars().enumerate() {
            if i >= max_len - 1 {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
    }
}

/// Top-level helper the rest of the app can call from teacher mode.
pub fn show_homework_dashboard(base_path: &Path) {
    match load_completed_homework(base_path) {
        Ok(list) => {
            println!("\n📊 Completed homework overview:");
            print_homework_table(&list);
        }
        Err(e) => {
            eprintln!("\n[ERROR] Could not load completed homework: {}\n", e);
        }
    }
}
