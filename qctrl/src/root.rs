//! Finding the project Q-CTRL works on. Kept apart from `main.rs` so it can be
//! tested without opening the window.

use std::fs::File;
use std::path::{Path, PathBuf};

/// The project root is the folder Q-CTRL was started from, the same way `git`
/// or `just` work. The binary itself can live anywhere: on PATH, in Downloads,
/// or at the project root.
pub fn project_root() -> PathBuf {
    std::env::current_dir().expect("Cannot determine the current working directory")
}

/// A Scaffold project has a `backlog/items/` folder at its root.
pub fn is_project(root: &Path) -> bool {
    root.join("backlog").join("items").is_dir()
}

/// The message shown when Q-CTRL is started outside a Scaffold project.
pub fn not_a_project_message(root: &Path) -> String {
    format!(
        "qctrl: no backlog/items/ folder in {}.\nRun qctrl from the root of a Scaffold project.",
        root.display()
    )
}

/// Open the debug log (`qctrl-debug.log` in the project root), but only inside
/// a Scaffold project, so Q-CTRL never leaves a log in a random folder.
/// Returns `None` outside a project, or if the file can't be opened.
pub fn open_debug_log(root: &Path) -> Option<File> {
    if !is_project(root) {
        return None;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("qctrl-debug.log"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qctrl-root-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_folder_with_backlog_items_is_a_project() {
        let dir = temp_dir("project");
        fs::create_dir_all(dir.join("backlog").join("items")).unwrap();
        assert!(is_project(&dir));
    }

    #[test]
    fn an_empty_folder_is_not_a_project() {
        let dir = temp_dir("empty");
        assert!(!is_project(&dir));
    }

    #[test]
    fn a_backlog_folder_without_items_is_not_a_project() {
        let dir = temp_dir("no-items");
        fs::create_dir_all(dir.join("backlog")).unwrap();
        assert!(!is_project(&dir));
    }

    #[test]
    fn a_file_named_items_does_not_count() {
        let dir = temp_dir("file-items");
        fs::create_dir_all(dir.join("backlog")).unwrap();
        fs::write(dir.join("backlog").join("items"), "not a folder").unwrap();
        assert!(!is_project(&dir));
    }

    #[test]
    fn a_subfolder_of_a_project_is_not_the_project() {
        // Like running from docs/ instead of the root: no walking up.
        let dir = temp_dir("subfolder");
        fs::create_dir_all(dir.join("backlog").join("items")).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        assert!(!is_project(&dir.join("docs")));
    }

    #[test]
    fn a_missing_folder_is_not_a_project() {
        let dir = temp_dir("missing");
        assert!(!is_project(&dir.join("does-not-exist")));
    }

    #[test]
    fn the_root_is_the_working_directory_not_the_binary_location() {
        assert_eq!(project_root(), std::env::current_dir().unwrap());
    }

    #[test]
    fn no_log_is_written_outside_a_project() {
        let dir = temp_dir("no-log");
        assert!(open_debug_log(&dir).is_none());
        assert!(!dir.join("qctrl-debug.log").exists());
    }

    #[test]
    fn the_log_is_written_in_the_project_root() {
        let dir = temp_dir("log");
        fs::create_dir_all(dir.join("backlog").join("items")).unwrap();
        assert!(open_debug_log(&dir).is_some());
        assert!(dir.join("qctrl-debug.log").is_file());
    }

    #[test]
    fn the_message_names_the_folder_and_the_fix() {
        let dir = temp_dir("message");
        let msg = not_a_project_message(&dir);
        assert!(msg.contains(&dir.display().to_string()));
        assert!(msg.contains("backlog/items/"));
        assert!(msg.contains("root of a Scaffold project"));
    }
}
