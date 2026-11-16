use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct TicketFrontmatter {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    #[serde(rename = "type", default)]
    pub ticket_type: String,
    #[serde(default)]
    pub order: f64,
    #[serde(default)]
    pub milestone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Ticket {
    pub frontmatter: TicketFrontmatter,
    pub body: String,
    /// Path this ticket was loaded from.
    pub path: std::path::PathBuf,
}

/// Parse a single ticket file. Returns None and prints a warning on error.
pub fn parse_ticket(path: &Path) -> Option<Ticket> {
    let content = fs::read_to_string(path)
        .map_err(|e| eprintln!("Warning: could not read {:?}: {}", path, e))
        .ok()?;

    let mut parts = content.splitn(3, "---");
    parts.next();
    let yaml_part = parts.next().map_err_warn(path, "missing frontmatter")?;
    let body = parts.next().unwrap_or("").trim().to_string();

    let frontmatter: TicketFrontmatter = serde_yaml::from_str(yaml_part)
        .map_err(|e| eprintln!("Warning: could not parse frontmatter in {:?}: {}", path, e))
        .ok()?;

    Some(Ticket {
        frontmatter,
        body,
        path: path.to_path_buf(),
    })
}

trait WarnOnErr<T> {
    fn map_err_warn(self, path: &Path, msg: &str) -> Option<T>;
}

impl<T> WarnOnErr<T> for Option<T> {
    fn map_err_warn(self, path: &Path, msg: &str) -> Option<T> {
        if self.is_none() {
            eprintln!("Warning: {:?}: {}", path, msg);
        }
        self
    }
}

/// Load all tickets from a directory, sorted by order then id.
pub fn load_tickets(dir: &Path) -> Vec<Ticket> {
    let mut tickets: Vec<Ticket> = walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("md")
                && e.file_type().is_file()
        })
        .filter_map(|e| parse_ticket(e.path()))
        .collect();

    tickets.sort_by(|a, b| {
        a.frontmatter
            .order
            .partial_cmp(&b.frontmatter.order)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.frontmatter.id.cmp(&b.frontmatter.id))
    });

    tickets
}

/// Extract the number from a ticket ID, but only if it belongs to `project_key`.
/// ("SB-007", "SB") → Some(7); ("APP-003", "SB") → None.
/// Each key has its own numbering continuity.
fn id_number(id: &str, project_key: &str) -> Option<u32> {
    id.strip_prefix(project_key)?.strip_prefix('-')?.parse().ok()
}

/// Read `project_key` from Scaffold's `.copier-answers.yml`.
/// Returns None if the file is missing, unparsable, or has no usable key.
pub fn read_project_key(answers_path: &Path) -> Option<String> {
    let content = fs::read_to_string(answers_path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let key = value.get("project_key")?.as_str()?.trim();
    if key.is_empty() { None } else { Some(key.to_string()) }
}

/// Default number of digits in a ticket ID number (`APP-001`), as in Scaffold.
pub const DEFAULT_TICKET_ID_DIGITS: usize = 3;

/// Read `ticket_id_digits` from Scaffold's `.copier-answers.yml`.
/// Falls back to 3 if the file is missing or unparsable, or the key is missing
/// or not a positive whole number.
pub fn read_ticket_id_digits(answers_path: &Path) -> usize {
    let digits = fs::read_to_string(answers_path)
        .ok()
        .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
        .and_then(|value| value.get("ticket_id_digits").and_then(|v| v.as_u64()));
    match digits {
        Some(n) if n >= 1 => n as usize,
        _ => DEFAULT_TICKET_ID_DIGITS,
    }
}

/// Scan items/ and archive/ for tickets with this project key and return the next ID
/// in that key's own sequence, zero-padded to `digits`. Tickets with other keys are
/// ignored, so a new key starts at 1. IDs written with a different width still count,
/// because only the number is compared.
pub fn next_ticket_id(project_key: &str, digits: usize, items_dir: &Path, archive_dir: &Path) -> String {
    let max_in = |dir: &Path| -> u32 {
        walkdir::WalkDir::new(dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("md")
                    && e.file_type().is_file()
            })
            .filter_map(|e| parse_ticket(e.path()))
            .filter_map(|t| id_number(&t.frontmatter.id, project_key))
            .max()
            .unwrap_or(0)
    };

    let max = max_in(items_dir).max(max_in(archive_dir));
    format!("{}-{:0width$}", project_key, max + 1, width = digits)
}

/// Create a new ticket file from the template, with id and status pre-filled.
pub fn create_ticket(
    id: &str,
    items_dir: &Path,
    template_path: &Path,
) -> Result<std::path::PathBuf, String> {
    let template = fs::read_to_string(template_path)
        .map_err(|e| format!("Cannot read template: {}", e))?;

    let content = template
        .replacen(
            &{
                let mut parts = template.splitn(3, "---");
                parts.next();
                let yaml = parts.next().unwrap_or("");
                serde_yaml::from_str::<serde_yaml::Value>(yaml)
                    .ok()
                    .and_then(|v| v["id"].as_str().map(String::from))
                    .unwrap_or_else(|| "APP-000".to_string())
            },
            id,
            1,
        )
        .replacen("status: OPEN", "status: OPEN", 1);

    let filename = format!("{}.md", id);
    let dest = items_dir.join(&filename);

    if dest.exists() {
        return Err(format!("File already exists: {:?}", dest));
    }

    fs::write(&dest, content).map_err(|e| format!("Cannot write {:?}: {}", dest, e))?;
    Ok(dest)
}

// ---------------------------------------------------------------------------
// Drag-and-drop reorder + status-change support
// ---------------------------------------------------------------------------

const REBALANCE_THRESHOLD: f64 = 0.001;

/// Write a new `status` value into a ticket's frontmatter on disk.
/// Rewrites only the `status:` line; all other content is preserved verbatim.
pub fn write_status(path: &Path, new_status: &str) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;

    let mut parts = content.splitn(3, "---");
    let _empty = parts.next().unwrap_or("");
    let yaml = parts
        .next()
        .ok_or_else(|| format!("No frontmatter in {:?}", path))?;
    let body = parts.next().unwrap_or("");

    let new_yaml = yaml
        .lines()
        .map(|line| {
            if line.starts_with("status:") {
                format!("status: {}", new_status)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let new_content = format!("---{}---{}", new_yaml, body);
    fs::write(path, new_content).map_err(|e| format!("Cannot write {:?}: {}", path, e))?;
    Ok(())
}

/// Write both `status` and `order` in a single pass to avoid two reads/writes.
pub fn write_status_and_order(path: &Path, new_status: &str, new_order: f64) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;

    let mut parts = content.splitn(3, "---");
    let _empty = parts.next().unwrap_or("");
    let yaml = parts
        .next()
        .ok_or_else(|| format!("No frontmatter in {:?}", path))?;
    let body = parts.next().unwrap_or("");

    let mut order_written = false;
    let new_yaml = yaml
        .lines()
        .map(|line| {
            if line.starts_with("status:") {
                format!("status: {}", new_status)
            } else if line.starts_with("order:") {
                order_written = true;
                format!("order: {:.6}", new_order)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let new_yaml = if order_written {
        new_yaml
    } else {
        format!("{}\norder: {:.6}", new_yaml.trim_end(), new_order)
    };

    let new_content = format!("---{}---{}", new_yaml, body);
    fs::write(path, new_content).map_err(|e| format!("Cannot write {:?}: {}", path, e))?;
    Ok(())
}

/// Write a new `order` value into a ticket's frontmatter on disk.
/// Rewrites only the `order:` line; all other content is preserved verbatim.
pub fn write_order(path: &Path, new_order: f64) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;

    // Replace the order: line inside the first frontmatter block.
    // Pattern: line starting with "order:" between the two "---" delimiters.
    let mut parts = content.splitn(3, "---");
    let _empty = parts.next().unwrap_or("");
    let yaml = parts
        .next()
        .ok_or_else(|| format!("No frontmatter in {:?}", path))?;
    let body = parts.next().unwrap_or("");

    // Replace or insert the order field
    let new_yaml = if yaml.contains("\norder:") || yaml.starts_with("order:") {
        // Replace existing line
        yaml.lines()
            .map(|line| {
                if line.starts_with("order:") {
                    format!("order: {:.6}", new_order)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Append before the closing content (rare: template might lack order)
        format!("{}\norder: {:.6}", yaml.trim_end(), new_order)
    };

    let new_content = format!("---{}---{}", new_yaml, body);
    // Ensure the yaml block starts with a newline after ---
    let new_content = if new_yaml.starts_with('\n') {
        new_content
    } else {
        format!("---\n{}---{}", new_yaml, body)
    };

    fs::write(path, new_content).map_err(|e| format!("Cannot write {:?}: {}", path, e))?;
    Ok(())
}

/// Edit a ticket's user-facing fields in one pass.
/// Only the listed fields are touched; all other frontmatter and the body
/// are preserved verbatim.
pub struct EditFields {
    pub title: String,
    pub priority: String,
    pub ticket_type: String,
    pub milestone: String,
    pub body: String,
}

pub fn write_fields(path: &Path, fields: &EditFields) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;

    let mut parts = content.splitn(3, "---");
    let _empty = parts.next().unwrap_or("");
    let yaml = parts
        .next()
        .ok_or_else(|| format!("No frontmatter in {:?}", path))?;
    // Ignore old body — we'll replace it wholesale.

    let new_yaml = yaml
        .lines()
        .map(|line| {
            if line.starts_with("title:") {
                format!("title: {}", fields.title)
            } else if line.starts_with("priority:") {
                format!("priority: {}", fields.priority)
            } else if line.starts_with("type:") {
                format!("type: {}", fields.ticket_type)
            } else if line.starts_with("milestone:") {
                format!("milestone: {}", fields.milestone)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Reconstruct: frontmatter + body (body always starts with a blank line)
    let new_body = if fields.body.is_empty() {
        String::new()
    } else {
        format!("\n{}", fields.body.trim_start_matches('\n'))
    };
    let new_content = format!("---{}---{}", new_yaml, new_body);

    fs::write(path, new_content).map_err(|e| format!("Cannot write {:?}: {}", path, e))?;
    Ok(())
}

/// Reorder a ticket within (or across, for SB-007) a column by computing a
/// midpoint `order` value.
///
/// `dragged_id`  — the ticket being dropped.
/// `after_id`    — id of the ticket it should appear after, or "" for top.
/// `column_tickets` — tickets currently visible in the target column, in
///                    display order (sorted by order asc).
/// `items_dir`   — used to locate the file to patch.
///
/// Returns the new order value assigned, or an error string.
pub fn reorder_ticket(
    dragged_id: &str,
    after_id: &str,
    column_tickets: &[Ticket],
    items_dir: &Path,
) -> Result<f64, String> {
    // Build ordered list excluding the dragged card itself
    let others: Vec<&Ticket> = column_tickets
        .iter()
        .filter(|t| t.frontmatter.id != dragged_id)
        .collect();

    // Find the position of `after_id` in others
    let insert_after_idx: Option<usize> = if after_id.is_empty() {
        None // insert at top
    } else {
        others
            .iter()
            .position(|t| t.frontmatter.id == after_id)
    };

    let new_order = compute_order(&others, insert_after_idx)?;

    // Find the ticket file and write
    let path = items_dir.join(format!("{}.md", dragged_id));
    if !path.exists() {
        return Err(format!("Ticket file not found: {:?}", path));
    }

    write_order(&path, new_order)?;

    // Check if rebalance is needed: if the gap between the written value and
    // either neighbour is below threshold, rebalance the whole column.
    let needs_rebalance = {
        let before_order = insert_after_idx.map(|i| others[i].frontmatter.order);
        let after_order = insert_after_idx
            .map(|i| i + 1)
            .unwrap_or(0)
            .checked_sub(0) // just to get after index
            .and_then(|_| {
                let after_idx = insert_after_idx.map(|i| i + 1).unwrap_or(0);
                others.get(after_idx).map(|t| t.frontmatter.order)
            });

        let gap_before = before_order.map(|o| (new_order - o).abs()).unwrap_or(f64::MAX);
        let gap_after = after_order.map(|o| (o - new_order).abs()).unwrap_or(f64::MAX);
        gap_before < REBALANCE_THRESHOLD || gap_after < REBALANCE_THRESHOLD
    };

    if needs_rebalance {
        rebalance_column(dragged_id, new_order, column_tickets, items_dir)?;
    }

    Ok(new_order)
}

/// Return the `order` value a ticket should receive when dropped after `after_id`
/// in a target column. Pass the target column's current tickets (the dragged
/// ticket may or may not be among them — it is excluded before computing).
/// `after_id` empty means "insert at top".
pub fn order_for_drop(after_id: &str, dragged_id: &str, column_tickets: &[Ticket]) -> f64 {
    let others: Vec<&Ticket> = column_tickets
        .iter()
        .filter(|t| t.frontmatter.id != dragged_id)
        .collect();

    let insert_after_idx: Option<usize> = if after_id.is_empty() {
        None
    } else {
        let pos = others.iter().position(|t| t.frontmatter.id == after_id);
        if pos.is_none() {
            // after_id not found in target column — it came from the source column.
            // Insert at bottom (after last card).
            return compute_order(&others, others.len().checked_sub(1)).unwrap_or(1.0);
        }
        pos
    };

    compute_order(&others, insert_after_idx).unwrap_or(1.0)
}

/// Compute the midpoint order value for inserting after `others[insert_after_idx]`.
/// `insert_after_idx = None` means insert at the top.
fn compute_order(others: &[&Ticket], insert_after_idx: Option<usize>) -> Result<f64, String> {
    let prev_order = insert_after_idx.map(|i| others[i].frontmatter.order);
    let next_order = insert_after_idx
        .map(|i| i + 1)
        .unwrap_or(0);
    let next_order = others.get(next_order).map(|t| t.frontmatter.order);

    let new_order = match (prev_order, next_order) {
        (None, None) => 1.0,                              // only card
        (None, Some(n)) => n - 1.0,                       // insert before first
        (Some(p), None) => p + 1.0,                       // insert after last
        (Some(p), Some(n)) => (p + n) / 2.0,              // midpoint
    };

    Ok(new_order)
}

/// Rebalance all order values in a column to evenly-spaced integers (1, 2, 3…).
/// The dragged ticket (identified by `dragged_id`) is placed at `target_order`
/// relative to the others; after rebalancing, all values become clean integers.
fn rebalance_column(
    dragged_id: &str,
    _target_order: f64,
    column_tickets: &[Ticket],
    items_dir: &Path,
) -> Result<(), String> {
    // Re-sort by current order to get the current sequence
    let mut sorted: Vec<&Ticket> = column_tickets.iter().collect();
    sorted.sort_by(|a, b| {
        a.frontmatter
            .order
            .partial_cmp(&b.frontmatter.order)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.frontmatter.id.cmp(&b.frontmatter.id))
    });

    for (i, ticket) in sorted.iter().enumerate() {
        let new_order = (i + 1) as f64;
        // Skip the dragged card — it was already written with a precise value;
        // this rebalance just cleans up the neighbours.
        if ticket.frontmatter.id == dragged_id {
            continue;
        }
        let path = items_dir.join(format!("{}.md", ticket.frontmatter.id));
        if path.exists() {
            let _ = write_order(&path, new_order); // best-effort; log but don't abort
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, content: Option<&str>) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("qctrl-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_file(&path);
        if let Some(c) = content {
            fs::write(&path, c).unwrap();
        }
        path
    }

    #[test]
    fn reads_project_key() {
        let p = temp_file("ok.yaml", Some("# comment\nproject_key: RC\n_commit: v1.0.0\n"));
        assert_eq!(read_project_key(&p), Some("RC".to_string()));
    }

    #[test]
    fn missing_file_gives_none() {
        let p = temp_file("missing.yaml", None);
        assert_eq!(read_project_key(&p), None);
    }

    #[test]
    fn missing_or_empty_key_gives_none() {
        let p = temp_file("nokey.yaml", Some("_commit: v1.0.0\n"));
        assert_eq!(read_project_key(&p), None);
        let p = temp_file("empty.yaml", Some("project_key: \"\"\n"));
        assert_eq!(read_project_key(&p), None);
    }

    #[test]
    fn invalid_yaml_gives_none() {
        let p = temp_file("bad.yaml", Some("project_key: [unclosed\n"));
        assert_eq!(read_project_key(&p), None);
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qctrl-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_ticket(dir: &Path, id: &str) {
        let content = format!("---\nid: {}\ntitle: t\nstatus: OPEN\ntype: feature\npriority: must\n---\n\nbody\n", id);
        fs::write(dir.join(format!("{}.md", id)), content).unwrap();
    }

    #[test]
    fn id_number_only_matches_own_key() {
        assert_eq!(id_number("SB-007", "SB"), Some(7));
        assert_eq!(id_number("APP-003", "SB"), None);
        assert_eq!(id_number("SB-007", "S"), None);
        assert_eq!(id_number("SB-007", "SBX"), None);
    }

    #[test]
    fn numbering_continues_within_a_key() {
        let items = temp_dir("items-a");
        let archive = temp_dir("archive-a");
        write_ticket(&items, "SB-001");
        write_ticket(&items, "SB-023");
        write_ticket(&archive, "SB-012");
        assert_eq!(next_ticket_id("SB", 3, &items, &archive), "SB-024");
    }

    #[test]
    fn new_key_starts_at_001() {
        let items = temp_dir("items-b");
        let archive = temp_dir("archive-b");
        write_ticket(&items, "SB-023");
        assert_eq!(next_ticket_id("APP", 3, &items, &archive), "APP-001");
    }

    #[test]
    fn keys_keep_separate_sequences() {
        let items = temp_dir("items-c");
        let archive = temp_dir("archive-c");
        write_ticket(&items, "SB-023");
        write_ticket(&items, "APP-001");
        write_ticket(&archive, "APP-004");
        assert_eq!(next_ticket_id("SB", 3, &items, &archive), "SB-024");
        assert_eq!(next_ticket_id("APP", 3, &items, &archive), "APP-005");
    }

    #[test]
    fn reads_ticket_id_digits() {
        let p = temp_file("digits-ok.yaml", Some("project_key: RC\nticket_id_digits: 4\n"));
        assert_eq!(read_ticket_id_digits(&p), 4);
    }

    #[test]
    fn ticket_id_digits_falls_back_to_3() {
        let p = temp_file("digits-missing-file.yaml", None);
        assert_eq!(read_ticket_id_digits(&p), 3);
        let p = temp_file("digits-nokey.yaml", Some("project_key: RC\n"));
        assert_eq!(read_ticket_id_digits(&p), 3);
        let p = temp_file("digits-bad-yaml.yaml", Some("ticket_id_digits: [unclosed\n"));
        assert_eq!(read_ticket_id_digits(&p), 3);
    }

    #[test]
    fn invalid_ticket_id_digits_fall_back_to_3() {
        for (name, content) in [
            ("digits-zero.yaml", "ticket_id_digits: 0\n"),
            ("digits-negative.yaml", "ticket_id_digits: -2\n"),
            ("digits-fraction.yaml", "ticket_id_digits: 3.5\n"),
            ("digits-text.yaml", "ticket_id_digits: four\n"),
            ("digits-empty.yaml", "ticket_id_digits:\n"),
        ] {
            let p = temp_file(name, Some(content));
            assert_eq!(read_ticket_id_digits(&p), 3, "{}", name);
        }
    }

    #[test]
    fn next_id_uses_the_configured_width() {
        let items = temp_dir("items-d");
        let archive = temp_dir("archive-d");
        write_ticket(&items, "SB-023");
        assert_eq!(next_ticket_id("SB", 4, &items, &archive), "SB-0024");
        assert_eq!(next_ticket_id("SB", 5, &items, &archive), "SB-00024");
        assert_eq!(next_ticket_id("SB", 1, &items, &archive), "SB-24");
    }

    #[test]
    fn mixed_widths_still_count_and_wide_numbers_grow() {
        let items = temp_dir("items-e");
        let archive = temp_dir("archive-e");
        write_ticket(&items, "SB-023");
        write_ticket(&items, "SB-0150");
        assert_eq!(next_ticket_id("SB", 3, &items, &archive), "SB-151");
        assert_eq!(next_ticket_id("SB", 4, &items, &archive), "SB-0151");
        write_ticket(&archive, "SB-999");
        write_ticket(&archive, "SB-1000");
        assert_eq!(next_ticket_id("SB", 3, &items, &archive), "SB-1001");
    }
}
