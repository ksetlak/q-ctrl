mod root;
mod ticket;

use std::sync::{Arc, Mutex};
use root::{is_project, not_a_project_message, open_debug_log, project_root};
use ticket::{apply_drop, create_ticket_in_column, load_tickets, next_ticket_id, write_fields, EditFields, Ticket};
use log::{debug, info, warn};

slint::include_modules!();

fn priority_colour(priority: &str) -> slint::Color {
    match priority {
        "must" => slint::Color::from_rgb_u8(224, 82, 82),
        "should" => slint::Color::from_rgb_u8(224, 169, 82),
        _ => slint::Color::from_rgb_u8(136, 136, 136),
    }
}

fn to_ticket_data(t: &Ticket) -> TicketData {
    TicketData {
        id: t.frontmatter.id.clone().into(),
        title: t.frontmatter.title.clone().into(),
        status: t.frontmatter.status.clone().into(),
        priority: t.frontmatter.priority.clone().into(),
        ticket_type: t.frontmatter.ticket_type.clone().into(),
        milestone: t.frontmatter.milestone.clone().unwrap_or_default().into(),
        body: t.body.clone().into(),
        priority_colour: priority_colour(&t.frontmatter.priority),
    }
}

fn filter_model(tickets: &[Ticket], status: &str) -> slint::ModelRc<TicketData> {
    let matching: Vec<TicketData> = tickets
        .iter()
        .filter(|t| t.frontmatter.status.to_uppercase() == status)
        .map(to_ticket_data)
        .collect();
    debug!("Column {:?}: {} tickets", status, matching.len());
    slint::ModelRc::new(slint::VecModel::from(matching))
}

fn populate_board(app: &AppWindow, tickets: &[Ticket]) {
    info!("Populating board with {} total tickets", tickets.len());
    app.set_tickets_open(filter_model(tickets, "OPEN"));
    app.set_tickets_ready(filter_model(tickets, "READY"));
    app.set_tickets_on_hold(filter_model(tickets, "ON-HOLD"));
    app.set_tickets_in_progress(filter_model(tickets, "IN_PROGRESS"));
    app.set_tickets_awaiting(filter_model(tickets, "AWAITING_OPERATOR"));
    app.set_tickets_canceled(filter_model(tickets, "CANCELED"));
    app.set_tickets_done(filter_model(tickets, "DONE"));
}

/// Resolve the ticket ID prefix.
/// 1. `project_key` in `.copier-answers.yml` at the repo root (source of truth, managed by Copier)
/// 2. the prefix of an existing ticket, if the config is missing or has no key
/// 3. `APP`, Scaffold's default
fn resolve_project_key(answers_path: &std::path::Path, tickets: &[Ticket]) -> String {
    if let Some(key) = ticket::read_project_key(answers_path) {
        return key;
    }
    tickets
        .iter()
        .filter_map(|t| t.frontmatter.id.split('-').next().map(String::from))
        .find(|k| !k.is_empty())
        .unwrap_or_else(|| {
            warn!("No project_key in .copier-answers.yml and no tickets to infer from — falling back to APP");
            "APP".to_string()
        })
}

fn main() {
    let root = project_root();

    // Logs go to qctrl-debug.log in the project root (gitignored), but only
    // inside a Scaffold project.
    let log_target = open_debug_log(&root).map(|f| env_logger::Target::Pipe(Box::new(f)));

    let mut logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("debug")
    );
    if let Some(target) = log_target {
        logger.target(target);
    }
    logger.init();

    info!("Q-CTRL starting");
    info!("Project root: {:?}", root);

    if !is_project(&root) {
        eprintln!("{}", not_a_project_message(&root));
        std::process::exit(1);
    }

    let items_dir = root.join("backlog").join("items");
    let archive_dir = root.join("backlog").join("archive");
    let template_path = root.join("backlog").join("_template.md");
    let answers_path = root.join(".copier-answers.yml");

    info!("items_dir:     {:?}", items_dir);
    info!("archive_dir:   {:?}", archive_dir);
    info!("template_path: {:?}", template_path);
    info!("answers_path:   {:?}", answers_path);

    let tickets = Arc::new(Mutex::new(load_tickets(&items_dir)));

    let app = AppWindow::new().expect("Failed to create application window");
    populate_board(&app, &tickets.lock().unwrap());

    // --- New item: requested (compute next ID) ---
    {
        let app_weak = app.as_weak();
        let items_dir_c = items_dir.clone();
        let archive_dir_c = archive_dir.clone();
        let answers_path_c = answers_path.clone();
        let tickets_c = tickets.clone();
        app.on_new_item_requested(move || {
            let locked = tickets_c.lock().unwrap();
            let key = resolve_project_key(&answers_path_c, &locked);
            info!("New item requested. Project key: {}", key);
            let digits = ticket::read_ticket_id_digits(&answers_path_c);
            let next_id = next_ticket_id(&key, digits, &items_dir_c, &archive_dir_c);
            info!("Next ID: {}", next_id);
            if let Some(app) = app_weak.upgrade() {
                app.set_dialog_next_id(next_id.into());
                app.set_dialog_error("".into());
            }
        });
    }

    // --- New item: confirmed (create file, reload) ---
    {
        let app_weak = app.as_weak();
        let items_dir_c = items_dir.clone();
        let archive_dir_c = archive_dir.clone();
        let answers_path_c = answers_path.clone();
        let template_path_c = template_path.clone();
        let tickets_c = tickets.clone();
        app.on_new_item_confirmed(move |title, priority| {
            let app = match app_weak.upgrade() {
                Some(a) => a,
                None => return,
            };

            let locked = tickets_c.lock().unwrap();
            let key = resolve_project_key(&answers_path_c, &locked);
            // New tickets start in the OPEN column and appear at its bottom.
            let open_column: Vec<Ticket> = locked
                .iter()
                .filter(|t| t.frontmatter.status.eq_ignore_ascii_case("OPEN"))
                .cloned()
                .collect();
            drop(locked);

            let digits = ticket::read_ticket_id_digits(&answers_path_c);
            let next_id = next_ticket_id(&key, digits, &items_dir_c, &archive_dir_c);
            info!("Creating ticket {} (title={:?}, priority={})", next_id, title, priority);

            match create_ticket_in_column(&next_id, &items_dir_c, &template_path_c, &open_column) {
                Ok(path) => {
                    info!("Created {:?}", path);
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let patched = content
                            .replacen(
                                "title: Short imperative title (≤ 100 characters)",
                                &format!("title: {}", title),
                                1,
                            )
                            .replacen("priority: must", &format!("priority: {}", priority), 1);
                        let _ = std::fs::write(&path, patched);
                    }
                    let fresh = load_tickets(&items_dir_c);
                    *tickets_c.lock().unwrap() = fresh;
                    populate_board(&app, &tickets_c.lock().unwrap());
                    app.set_dialog_visible(false);
                }
                Err(e) => {
                    warn!("Failed to create ticket: {}", e);
                    app.set_dialog_error(e.into());
                }
            }
        });
    }

    // --- New item: cancelled ---
    {
        let app_weak = app.as_weak();
        app.on_new_item_cancelled(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_dialog_visible(false);
            }
        });
    }

    // --- Drag: ghost ticket lookup ---
    {
        let app_weak = app.as_weak();
        let tickets_c = tickets.clone();
        app.on_drag_started_rust(move |id| {
            let locked = tickets_c.lock().unwrap();
            if let Some(t) = locked.iter().find(|t| t.frontmatter.id == id.as_str()) {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drag_ghost_ticket(to_ticket_data(t));
                }
            }
        });
    }

    // --- Drag-and-drop reorder ---
    {
        let app_weak = app.as_weak();
        let items_dir_c = items_dir.clone();
        let tickets_c = tickets.clone();
        app.on_reorder_ticket(move |dragged_id, after_id, column_status| {
            let app = match app_weak.upgrade() {
                Some(a) => a,
                None => return,
            };

            let dragged_id = dragged_id.to_string();
            let after_id = after_id.to_string();
            let column_status = column_status.to_string();

            info!(
                "reorder-ticket: dragged={}, after={:?}, column={}",
                dragged_id,
                if after_id.is_empty() { "<top>" } else { &after_id },
                column_status
            );

            let locked = tickets_c.lock().unwrap();

            // Tickets currently in the target column, in display order.
            let column_tickets: Vec<Ticket> = locked
                .iter()
                .filter(|t| t.frontmatter.status.eq_ignore_ascii_case(&column_status))
                .cloned()
                .collect();
            drop(locked);

            // Every drop, within a column or across columns, ends with a real
            // order that matches where the card was dropped.
            match apply_drop(&items_dir_c, &dragged_id, &after_id, &column_status, &column_tickets) {
                Ok(order) => {
                    info!("Dropped {} in {} @ {:.6}", dragged_id, column_status, order);
                    let fresh = load_tickets(&items_dir_c);
                    *tickets_c.lock().unwrap() = fresh;
                    populate_board(&app, &tickets_c.lock().unwrap());
                }
                Err(e) => warn!("Drop failed: {}", e),
            }
        });
    }

    // Fixed model arrays — must match the ComboBox model order in app.slint exactly.
    const PRIORITY_MODEL: &[&str] = &["must", "should", "could"];
    const TYPE_MODEL: &[&str] = &["feature", "bug", "refactor", "perf", "security", "chore", "docs", "test", "setup", "infra"];

    fn priority_index(s: &str) -> i32 {
        PRIORITY_MODEL.iter().position(|&v| v == s).unwrap_or(0) as i32
    }

    fn type_index(s: &str) -> i32 {
        TYPE_MODEL.iter().position(|&v| v == s).unwrap_or(0) as i32
    }

    // --- Card clicked: open edit dialog ---
    {
        let app_weak = app.as_weak();
        let tickets_c = tickets.clone();
        app.on_card_clicked(move |id| {
            let app = match app_weak.upgrade() {
                Some(a) => a,
                None => return,
            };
            let locked = tickets_c.lock().unwrap();
            if let Some(t) = locked.iter().find(|t| t.frontmatter.id == id.as_str()) {
                app.set_edit_id(t.frontmatter.id.clone().into());
                app.set_edit_status(t.frontmatter.status.clone().into());
                app.set_edit_title(t.frontmatter.title.clone().into());
                app.set_edit_priority_index(priority_index(&t.frontmatter.priority));
                app.set_edit_type_index(type_index(&t.frontmatter.ticket_type));
                app.set_edit_milestone(t.frontmatter.milestone.clone().unwrap_or_default().into());
                app.set_edit_body(t.body.clone().into());
                app.set_edit_error("".into());
                app.set_edit_visible(true);
            }
        });
    }

    // --- Edit confirmed: resolve indices → strings, write fields, reload board ---
    {
        let app_weak = app.as_weak();
        let items_dir_c = items_dir.clone();
        let tickets_c = tickets.clone();
        app.on_edit_confirmed(move |id, title, priority_idx, type_idx, milestone, body| {
            let app = match app_weak.upgrade() {
                Some(a) => a,
                None => return,
            };
            let priority = PRIORITY_MODEL.get(priority_idx as usize).copied().unwrap_or("must");
            let ticket_type = TYPE_MODEL.get(type_idx as usize).copied().unwrap_or("feature");
            let path = items_dir_c.join(format!("{}.md", id));
            let fields = EditFields {
                title: title.to_string(),
                priority: priority.to_string(),
                ticket_type: ticket_type.to_string(),
                milestone: milestone.to_string(),
                body: body.to_string(),
            };
            match write_fields(&path, &fields) {
                Ok(()) => {
                    info!("Edited ticket {} (priority={}, type={})", id, priority, ticket_type);
                    let fresh = load_tickets(&items_dir_c);
                    *tickets_c.lock().unwrap() = fresh;
                    populate_board(&app, &tickets_c.lock().unwrap());
                    app.set_edit_visible(false);
                }
                Err(e) => {
                    warn!("Edit failed for {}: {}", id, e);
                    app.set_edit_error(e.into());
                }
            }
        });
    }

    // --- Edit cancelled ---
    {
        let app_weak = app.as_weak();
        app.on_edit_cancelled(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_edit_visible(false);
            }
        });
    }

    app.run().expect("Failed to run application");
}
