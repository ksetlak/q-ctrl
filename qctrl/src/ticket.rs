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
    pub order: Option<f64>,
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
        compare_order(a.frontmatter.order, b.frontmatter.order)
            .then(a.frontmatter.id.cmp(&b.frontmatter.id))
    });

    tickets
}

/// Compare two optional order values: ordered tickets come first (lowest value
/// first), unordered ones (no value) come after them. `0.0` is a normal value.
pub fn compare_order(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Id of the ticket that shows last in a column, or "" for an empty column.
/// Pass it as `after_id` to put a ticket at the bottom.
pub fn last_in_column(column_tickets: &[Ticket]) -> String {
    let mut sorted: Vec<&Ticket> = column_tickets.iter().collect();
    sorted.sort_by(|a, b| {
        compare_order(a.frontmatter.order, b.frontmatter.order)
            .then(a.frontmatter.id.cmp(&b.frontmatter.id))
    });
    sorted
        .last()
        .map(|t| t.frontmatter.id.clone())
        .unwrap_or_default()
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

/// Create a ticket from the template and place it at the bottom of its column
/// (the OPEN column, where new tickets start). A ticket made on the board
/// always appears somewhere, so it always gets an order that matches. The
/// template itself carries no `order`, which keeps tickets created by hand or
/// by an agent unordered.
pub fn create_ticket_in_column(
    id: &str,
    items_dir: &Path,
    template_path: &Path,
    column_tickets: &[Ticket],
) -> Result<std::path::PathBuf, String> {
    let path = create_ticket(id, items_dir, template_path)?;
    let after_id = last_in_column(column_tickets);
    let writes = place_in_column(&after_id, id, column_tickets);
    for (ticket_id, order) in &writes {
        let target = items_dir.join(format!("{}.md", ticket_id));
        write_order(&target, *order)?;
    }
    Ok(path)
}

// ---------------------------------------------------------------------------
// Drag-and-drop placement, status changes, and ticket file writers
// ---------------------------------------------------------------------------

/// Gap below which two neighbouring order values are considered too close.
const REBALANCE_THRESHOLD: f64 = 0.001;

/// Rebuild a ticket file from its frontmatter lines and body. `yaml_lines`
/// come from splitting the text between the two `---` delimiters, so the first
/// line is empty. The closing delimiter always goes on its own line.
fn assemble(yaml_lines: &[String], body: &str) -> String {
    format!("---{}\n---{}", yaml_lines.join("\n"), body)
}

/// Split a ticket file into its frontmatter lines and body.
fn split_ticket(content: &str, path: &Path) -> Result<(Vec<String>, String), String> {
    let mut parts = content.splitn(3, "---");
    let _empty = parts.next().unwrap_or("");
    let yaml = parts
        .next()
        .ok_or_else(|| format!("No frontmatter in {:?}", path))?;
    let body = parts.next().unwrap_or("");
    Ok((yaml.lines().map(String::from).collect(), body.to_string()))
}

/// Write a new `status` value into a ticket's frontmatter on disk.
/// Rewrites only the `status:` line; all other content is preserved verbatim.
pub fn write_status(path: &Path, new_status: &str) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
    let (lines, body) = split_ticket(&content, path)?;

    let new_lines: Vec<String> = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("status:") {
                format!("status: {}", new_status)
            } else {
                line
            }
        })
        .collect();

    fs::write(path, assemble(&new_lines, &body))
        .map_err(|e| format!("Cannot write {:?}: {}", path, e))
}

/// Set `order` in a list of frontmatter lines: replace the line if the ticket
/// has one, otherwise add it at the end.
fn set_order_line(lines: Vec<String>, new_order: f64) -> Vec<String> {
    let mut written = false;
    let mut out: Vec<String> = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("order:") {
                written = true;
                format!("order: {:.6}", new_order)
            } else {
                line
            }
        })
        .collect();
    if !written {
        while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
            out.pop();
        }
        out.push(format!("order: {:.6}", new_order));
    }
    out
}

/// Write both `status` and `order` in a single pass to avoid two reads/writes.
pub fn write_status_and_order(path: &Path, new_status: &str, new_order: f64) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
    let (lines, body) = split_ticket(&content, path)?;

    let with_status: Vec<String> = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("status:") {
                format!("status: {}", new_status)
            } else {
                line
            }
        })
        .collect();

    let new_lines = set_order_line(with_status, new_order);
    fs::write(path, assemble(&new_lines, &body))
        .map_err(|e| format!("Cannot write {:?}: {}", path, e))
}

/// Write a new `order` value into a ticket's frontmatter on disk.
/// Replaces the `order:` line, or adds one when the ticket has none. All other
/// content is preserved verbatim.
pub fn write_order(path: &Path, new_order: f64) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
    let (lines, body) = split_ticket(&content, path)?;
    let new_lines = set_order_line(lines, new_order);
    fs::write(path, assemble(&new_lines, &body))
        .map_err(|e| format!("Cannot write {:?}: {}", path, e))
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
    // The old body is ignored: it is replaced wholesale.
    let (lines, _old_body) = split_ticket(&content, path)?;

    let new_lines: Vec<String> = lines
        .into_iter()
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
                line
            }
        })
        .collect();

    // Body always starts with a blank line.
    let new_body = if fields.body.is_empty() {
        String::new()
    } else {
        format!("\n{}", fields.body.trim_start_matches('\n'))
    };

    fs::write(path, assemble(&new_lines, &new_body))
        .map_err(|e| format!("Cannot write {:?}: {}", path, e))
}

/// Decide the order values to write when `dragged_id` is placed in a column.
///
/// `after_id` is the ticket it should appear after, or "" for the top of the
/// column. An id that isn't in the column means "at the bottom". The dragged
/// ticket may or may not already be in `column_tickets`.
///
/// The result lists `(ticket id, order)` pairs to write. It always contains
/// the dragged ticket, so a placement made on the board never leaves a ticket
/// without an order. Tickets without an order sort after ordered ones, so when
/// the ticket lands below some of them, those get orders too. That keeps the
/// saved order the same as what the board shows.
///
/// `0.0` is an ordinary value here. Only a missing `order` means unordered.
pub fn place_in_column(after_id: &str, dragged_id: &str, column_tickets: &[Ticket]) -> Vec<(String, f64)> {
    let mut others: Vec<&Ticket> = column_tickets
        .iter()
        .filter(|t| t.frontmatter.id != dragged_id)
        .collect();
    others.sort_by(|a, b| {
        compare_order(a.frontmatter.order, b.frontmatter.order)
            .then(a.frontmatter.id.cmp(&b.frontmatter.id))
    });

    // Insert position: 0 is the top, k + 1 is just after others[k].
    let idx = if after_id.is_empty() {
        0
    } else {
        others
            .iter()
            .position(|t| t.frontmatter.id == after_id)
            .map(|p| p + 1)
            .unwrap_or(others.len())
    };

    // Ordered tickets come first, so this is the number of leading ones.
    let ordered = others.iter().take_while(|t| t.frontmatter.order.is_some()).count();

    let mut writes: Vec<(String, f64)> = Vec::new();
    let new_order = if idx <= ordered {
        let prev = if idx > 0 { others[idx - 1].frontmatter.order } else { None };
        let next = if idx < ordered { others[idx].frontmatter.order } else { None };
        match (prev, next) {
            (None, None) => 1.0,
            (None, Some(n)) => n - 1.0,
            (Some(p), None) => p + 1.0,
            (Some(p), Some(n)) => (p + n) / 2.0,
        }
    } else {
        // Landing below unordered tickets: number them after the last ordered one.
        let base = if ordered > 0 {
            others[ordered - 1].frontmatter.order.unwrap_or(0.0)
        } else {
            0.0
        };
        for (n, t) in others[ordered..idx].iter().enumerate() {
            writes.push((t.frontmatter.id.clone(), base + (n + 1) as f64));
        }
        base + (idx - ordered + 1) as f64
    };
    writes.push((dragged_id.to_string(), new_order));

    // The column as it will look: every ticket that has an order, top to bottom.
    let mut sequence: Vec<(String, Option<f64>)> = others
        .iter()
        .map(|t| {
            let order = writes
                .iter()
                .find(|(id, _)| *id == t.frontmatter.id)
                .map(|(_, o)| *o)
                .or(t.frontmatter.order);
            (t.frontmatter.id.clone(), order)
        })
        .collect();
    sequence.insert(idx, (dragged_id.to_string(), Some(new_order)));
    let ordered_part: Vec<&(String, Option<f64>)> = sequence
        .iter()
        .take_while(|(_, o)| o.is_some())
        .collect();

    // Neighbours that ended up too close together (or equal): renumber 1, 2, 3...
    let crowded = ordered_part.windows(2).any(|w| {
        let a = w[0].1.unwrap_or(0.0);
        let b = w[1].1.unwrap_or(0.0);
        (b - a).abs() < REBALANCE_THRESHOLD
    });
    if crowded {
        return ordered_part
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (id.clone(), (i + 1) as f64))
            .collect();
    }

    writes
}

/// Place a ticket in a column and save the result: the ticket's own file gets
/// its new status and order, and any other file whose order has to change gets
/// that too. Returns the order written for the dragged ticket.
pub fn apply_drop(
    items_dir: &Path,
    dragged_id: &str,
    after_id: &str,
    status: &str,
    column_tickets: &[Ticket],
) -> Result<f64, String> {
    let writes = place_in_column(after_id, dragged_id, column_tickets);

    let dragged_order = writes
        .iter()
        .find(|(id, _)| id == dragged_id)
        .map(|(_, o)| *o)
        .ok_or_else(|| format!("No order computed for {}", dragged_id))?;

    // The dragged ticket first: if its file is missing, nothing else is touched.
    let dragged_path = items_dir.join(format!("{}.md", dragged_id));
    if !dragged_path.exists() {
        return Err(format!("Ticket file not found: {:?}", dragged_path));
    }
    write_status_and_order(&dragged_path, status, dragged_order)?;

    for (id, order) in writes.iter().filter(|(id, _)| id != dragged_id) {
        let path = items_dir.join(format!("{}.md", id));
        write_order(&path, *order)?;
    }

    Ok(dragged_order)
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

    // --- order handling (SB-025) -------------------------------------------

    /// Write a ticket with an optional order line. None leaves the key out.
    fn write_ordered(dir: &Path, id: &str, status: &str, order: Option<&str>) {
        let order_line = order.map(|o| format!("order: {}\n", o)).unwrap_or_default();
        let content = format!(
            "---\nid: {}\ntitle: t\nstatus: {}\ntype: feature\npriority: must\n{}---\n\nbody\n",
            id, status, order_line
        );
        fs::write(dir.join(format!("{}.md", id)), content).unwrap();
    }

    fn column(dir: &Path, status: &str) -> Vec<Ticket> {
        load_tickets(dir)
            .into_iter()
            .filter(|t| t.frontmatter.status == status)
            .collect()
    }

    fn ids(tickets: &[Ticket]) -> Vec<String> {
        tickets.iter().map(|t| t.frontmatter.id.clone()).collect()
    }

    fn order_of(dir: &Path, id: &str) -> Option<f64> {
        parse_ticket(&dir.join(format!("{}.md", id))).unwrap().frontmatter.order
    }

    #[test]
    fn missing_empty_and_null_order_load_as_unordered() {
        let dir = temp_dir("ord-load");
        write_ordered(&dir, "A-001", "OPEN", None);
        write_ordered(&dir, "A-002", "OPEN", Some(""));
        write_ordered(&dir, "A-003", "OPEN", Some("null"));
        write_ordered(&dir, "A-004", "OPEN", Some("0.0"));
        assert_eq!(order_of(&dir, "A-001"), None);
        assert_eq!(order_of(&dir, "A-002"), None);
        assert_eq!(order_of(&dir, "A-003"), None);
        assert_eq!(order_of(&dir, "A-004"), Some(0.0));
    }

    #[test]
    fn unordered_tickets_sort_after_ordered_ones_then_by_id() {
        let dir = temp_dir("ord-sort");
        write_ordered(&dir, "A-001", "OPEN", None);
        write_ordered(&dir, "A-002", "OPEN", Some("5.0"));
        write_ordered(&dir, "A-003", "OPEN", Some("0.0"));
        write_ordered(&dir, "A-004", "OPEN", None);
        write_ordered(&dir, "A-005", "OPEN", Some("-2.0"));
        assert_eq!(
            ids(&load_tickets(&dir)),
            vec!["A-005", "A-003", "A-002", "A-001", "A-004"]
        );
    }

    #[test]
    fn zero_is_an_ordinary_order() {
        // The original bug: dropping above a card with order 1.0 computed exactly 0.0.
        let dir = temp_dir("ord-zero");
        write_ordered(&dir, "A-001", "READY", Some("1.0"));
        write_ordered(&dir, "A-002", "OPEN", Some("3.0"));
        let col = column(&dir, "READY");
        let order = apply_drop(&dir, "A-002", "", "READY", &col).unwrap();
        assert_eq!(order, 0.0);
        assert_eq!(order_of(&dir, "A-002"), Some(0.0));
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-002", "A-001"]);
    }

    #[test]
    fn midpoint_of_minus_one_and_one_is_kept() {
        let dir = temp_dir("ord-mid");
        write_ordered(&dir, "A-001", "READY", Some("-1.0"));
        write_ordered(&dir, "A-002", "READY", Some("1.0"));
        write_ordered(&dir, "A-003", "OPEN", None);
        let col = column(&dir, "READY");
        apply_drop(&dir, "A-003", "A-001", "READY", &col).unwrap();
        assert_eq!(order_of(&dir, "A-003"), Some(0.0));
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-001", "A-003", "A-002"]);
    }

    #[test]
    fn drop_into_an_empty_column() {
        let dir = temp_dir("ord-empty");
        write_ordered(&dir, "A-001", "OPEN", None);
        let col = column(&dir, "READY");
        assert!(col.is_empty());
        apply_drop(&dir, "A-001", "", "READY", &col).unwrap();
        assert_eq!(order_of(&dir, "A-001"), Some(1.0));
        assert_eq!(column(&dir, "READY").len(), 1);
    }

    #[test]
    fn drop_at_top_bottom_and_between() {
        let dir = temp_dir("ord-pos");
        write_ordered(&dir, "A-001", "READY", Some("1.0"));
        write_ordered(&dir, "A-002", "READY", Some("2.0"));
        write_ordered(&dir, "A-003", "OPEN", None);
        write_ordered(&dir, "A-004", "OPEN", None);
        write_ordered(&dir, "A-005", "OPEN", None);

        let col = column(&dir, "READY");
        apply_drop(&dir, "A-003", "", "READY", &col).unwrap();
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-003", "A-001", "A-002"]);

        let col = column(&dir, "READY");
        apply_drop(&dir, "A-004", "A-002", "READY", &col).unwrap();
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-003", "A-001", "A-002", "A-004"]);

        let col = column(&dir, "READY");
        apply_drop(&dir, "A-005", "A-001", "READY", &col).unwrap();
        assert_eq!(
            ids(&column(&dir, "READY")),
            vec!["A-003", "A-001", "A-005", "A-002", "A-004"]
        );
    }

    #[test]
    fn no_drop_leaves_a_ticket_without_an_order() {
        let dir = temp_dir("ord-never-none");
        write_ordered(&dir, "A-001", "READY", None);
        write_ordered(&dir, "A-002", "READY", None);
        for after in ["", "A-001", "A-002", "unknown-id"] {
            write_ordered(&dir, "A-003", "OPEN", None);
            let col = column(&dir, "READY");
            apply_drop(&dir, "A-003", after, "READY", &col).unwrap();
            assert!(order_of(&dir, "A-003").is_some(), "after={:?}", after);
        }
    }

    #[test]
    fn dropping_below_unordered_tickets_numbers_them_so_the_view_matches() {
        let dir = temp_dir("ord-below-unordered");
        write_ordered(&dir, "A-001", "READY", Some("4.0"));
        write_ordered(&dir, "A-002", "READY", None);
        write_ordered(&dir, "A-003", "READY", None);
        write_ordered(&dir, "A-004", "OPEN", None);
        let before = column(&dir, "READY");
        assert_eq!(ids(&before), vec!["A-001", "A-002", "A-003"]);
        // Drop between the two unordered tickets.
        apply_drop(&dir, "A-004", "A-002", "READY", &before).unwrap();
        assert_eq!(
            ids(&column(&dir, "READY")),
            vec!["A-001", "A-002", "A-004", "A-003"]
        );
        assert!(order_of(&dir, "A-002").is_some());
        assert!(order_of(&dir, "A-004").is_some());
    }

    #[test]
    fn same_column_move_keeps_the_rest_in_place() {
        let dir = temp_dir("ord-same-col");
        write_ordered(&dir, "A-001", "READY", Some("1.0"));
        write_ordered(&dir, "A-002", "READY", Some("2.0"));
        write_ordered(&dir, "A-003", "READY", Some("3.0"));
        let col = column(&dir, "READY");
        apply_drop(&dir, "A-001", "A-003", "READY", &col).unwrap();
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-002", "A-003", "A-001"]);
        assert_eq!(order_of(&dir, "A-002"), Some(2.0));
        assert_eq!(order_of(&dir, "A-003"), Some(3.0));
    }

    #[test]
    fn crowded_neighbours_are_renumbered_in_the_same_order() {
        let dir = temp_dir("ord-crowded");
        write_ordered(&dir, "A-001", "READY", Some("1.0"));
        write_ordered(&dir, "A-002", "READY", Some("1.0005"));
        write_ordered(&dir, "A-003", "OPEN", None);
        let col = column(&dir, "READY");
        apply_drop(&dir, "A-003", "A-001", "READY", &col).unwrap();
        assert_eq!(ids(&column(&dir, "READY")), vec!["A-001", "A-003", "A-002"]);
        assert_eq!(order_of(&dir, "A-001"), Some(1.0));
        assert_eq!(order_of(&dir, "A-003"), Some(2.0));
        assert_eq!(order_of(&dir, "A-002"), Some(3.0));
    }

    #[test]
    fn a_missing_dragged_file_changes_nothing() {
        let dir = temp_dir("ord-missing");
        write_ordered(&dir, "A-001", "READY", Some("1.0"));
        let col = column(&dir, "READY");
        assert!(apply_drop(&dir, "A-999", "", "READY", &col).is_err());
        assert_eq!(order_of(&dir, "A-001"), Some(1.0));
    }

    fn template_file(dir: &Path) -> std::path::PathBuf {
        let p = dir.join("_template.md");
        fs::write(
            &p,
            "---\nid: SB-000\ntitle: Short imperative title (≤ 100 characters)\nstatus: OPEN\ntype: feature\npriority: must\nmilestone: M1\ndependencies: []\nreferences:\n  adrs: []\n---\n\n## Description\n",
        )
        .unwrap();
        p
    }

    #[test]
    fn a_ticket_created_in_the_gui_goes_to_the_bottom_of_open() {
        let dir = temp_dir("ord-create");
        let tpl_dir = temp_dir("ord-create-tpl");
        let template = template_file(&tpl_dir);
        write_ordered(&dir, "SB-001", "OPEN", Some("2.0"));
        write_ordered(&dir, "SB-002", "OPEN", None);
        write_ordered(&dir, "SB-003", "READY", Some("1.0"));
        let open = column(&dir, "OPEN");
        create_ticket_in_column("SB-004", &dir, &template, &open).unwrap();
        assert!(order_of(&dir, "SB-004").is_some());
        assert_eq!(ids(&column(&dir, "OPEN")), vec!["SB-001", "SB-002", "SB-004"]);
        // The unordered ticket above it got a number so the view matches the files.
        assert!(order_of(&dir, "SB-002").is_some());
    }

    #[test]
    fn a_ticket_created_in_an_empty_open_column_is_ordered() {
        let dir = temp_dir("ord-create-empty");
        let tpl_dir = temp_dir("ord-create-empty-tpl");
        let template = template_file(&tpl_dir);
        create_ticket_in_column("SB-001", &dir, &template, &[]).unwrap();
        assert_eq!(order_of(&dir, "SB-001"), Some(1.0));
    }

    #[test]
    fn a_ticket_made_from_the_template_alone_stays_unordered() {
        let dir = temp_dir("ord-tpl-only");
        let tpl_dir = temp_dir("ord-tpl-only-tpl");
        let template = template_file(&tpl_dir);
        create_ticket("SB-001", &dir, &template).unwrap();
        assert_eq!(order_of(&dir, "SB-001"), None);
    }

    #[test]
    fn writers_keep_the_closing_delimiter_on_its_own_line() {
        let dir = temp_dir("ord-delims");
        write_ordered(&dir, "A-001", "OPEN", Some("1.0"));
        let path = dir.join("A-001.md");

        write_order(&path, 7.0).unwrap();
        write_status(&path, "READY").unwrap();
        write_status_and_order(&path, "DONE", 9.0).unwrap();
        write_fields(
            &path,
            &EditFields {
                title: "New".into(),
                priority: "should".into(),
                ticket_type: "bug".into(),
                milestone: "M2".into(),
                body: "Body text".into(),
            },
        )
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\n---\n"), "closing delimiter on its own line: {:?}", text);
        assert!(!text.contains("---\n---"), "no doubled delimiter: {:?}", text);
        let t = parse_ticket(&path).unwrap();
        assert_eq!(t.frontmatter.status, "DONE");
        assert_eq!(t.frontmatter.order, Some(9.0));
        assert_eq!(t.frontmatter.title, "New");
        assert_eq!(t.body, "Body text");
    }

    #[test]
    fn writing_an_order_adds_the_key_when_the_ticket_has_none() {
        let dir = temp_dir("ord-add-key");
        write_ordered(&dir, "A-001", "OPEN", None);
        let path = dir.join("A-001.md");
        write_order(&path, 3.0).unwrap();
        assert_eq!(order_of(&dir, "A-001"), Some(3.0));
        assert_eq!(parse_ticket(&path).unwrap().body, "body");
    }
}
