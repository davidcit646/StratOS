use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Flat,
    Tree,
}

#[derive(Clone, Debug)]
pub struct BrowserEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_parent_row: bool,
}

#[derive(Clone, Debug)]
pub enum PreviewKind {
    FolderSummary(String),
    TextSnippet(String),
    ScriptHint(String),
    ConfigSummary(String),
    BinaryHint(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoubleClickAction {
    NavigateDirectory,
    RunScriptConfirm,
    OpenConfigEditor,
    OpenWithXdg,
}

#[derive(Debug)]
pub struct FileBrowser {
    cwd: PathBuf,
    view_mode: ViewMode,
    expanded: HashSet<PathBuf>,
    entries: Vec<BrowserEntry>,
}

impl FileBrowser {
    pub fn new(cwd: PathBuf) -> Self {
        let mut browser = Self {
            cwd,
            view_mode: ViewMode::Flat,
            expanded: HashSet::new(),
            entries: Vec::new(),
        };
        browser.refresh();
        browser
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn entries(&self) -> &[BrowserEntry] {
        &self.entries
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Flat => ViewMode::Tree,
            ViewMode::Tree => ViewMode::Flat,
        };
        self.refresh();
    }

    pub fn navigate_to(&mut self, path: PathBuf) -> bool {
        if !path.is_dir() {
            return false;
        }
        if self.cwd == path {
            return true;
        }
        self.cwd = path;
        self.refresh();
        true
    }

    pub fn go_up(&mut self) -> bool {
        let parent = match self.cwd.parent() {
            Some(value) => value.to_path_buf(),
            None => return false,
        };
        self.navigate_to(parent)
    }

    pub fn toggle_expand(&mut self, path: &Path) {
        let key = path.to_path_buf();
        if self.expanded.contains(&key) {
            self.expanded.remove(&key);
        } else {
            self.expanded.insert(key);
        }
        if self.view_mode == ViewMode::Tree {
            self.refresh();
        }
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        if let Some(parent) = self.cwd.parent() {
            self.entries.push(BrowserEntry {
                path: parent.to_path_buf(),
                name: ".. (go up)".to_string(),
                is_dir: true,
                depth: 0,
                is_parent_row: true,
            });
        }

        match self.view_mode {
            ViewMode::Flat => self.build_flat(),
            ViewMode::Tree => self.build_tree(),
        }
    }

    fn build_flat(&mut self) {
        for child in sorted_children(&self.cwd) {
            self.entries.push(BrowserEntry {
                name: display_name(&child),
                is_dir: child.is_dir(),
                path: child,
                depth: 0,
                is_parent_row: false,
            });
        }
    }

    fn build_tree(&mut self) {
        for child in sorted_children(&self.cwd) {
            self.push_tree_entry(child, 0);
        }
    }

    fn push_tree_entry(&mut self, path: PathBuf, depth: usize) {
        let is_dir = path.is_dir();
        self.entries.push(BrowserEntry {
            name: display_name(&path),
            is_dir,
            path: path.clone(),
            depth,
            is_parent_row: false,
        });

        if !is_dir {
            return;
        }
        if !self.expanded.contains(&path) {
            return;
        }

        let recurse = fs::symlink_metadata(&path)
            .map(|meta| !meta.file_type().is_symlink())
            .unwrap_or(false);
        if !recurse {
            return;
        }
        for child in sorted_children(&path) {
            self.push_tree_entry(child, depth + 1);
        }
    }

    pub fn preview_for(&self, path: &Path) -> PreviewKind {
        if path.is_dir() {
            let mut dirs = 0usize;
            let mut files = 0usize;
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        dirs += 1;
                    } else {
                        files += 1;
                    }
                }
            }
            return PreviewKind::FolderSummary(format!("{} dirs, {} files", dirs, files));
        }

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if is_config_name(&name) {
            return PreviewKind::ConfigSummary(format!("Config file: {}", path.display()));
        }
        if is_script_name(&name) {
            return PreviewKind::ScriptHint(format!("Script detected: {}", path.display()));
        }

        match read_text_preview(path, 8) {
            Some(preview) => PreviewKind::TextSnippet(preview),
            None => PreviewKind::BinaryHint(format!("Binary/unknown file: {}", path.display())),
        }
    }

    pub fn action_for_double_click(&self, path: &Path) -> DoubleClickAction {
        if path.is_dir() {
            return DoubleClickAction::NavigateDirectory;
        }

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if is_script_name(&name) {
            return DoubleClickAction::RunScriptConfirm;
        }
        if is_config_name(&name) {
            return DoubleClickAction::OpenConfigEditor;
        }
        DoubleClickAction::OpenWithXdg
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn sorted_children(path: &Path) -> Vec<PathBuf> {
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            children.push(entry.path());
        }
    }
    children.sort_by(|left, right| compare_entries(left, right));
    children
}

fn compare_entries(left: &Path, right: &Path) -> Ordering {
    match (left.is_dir(), right.is_dir()) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => {
            let left_name = display_name(left).to_ascii_lowercase();
            let right_name = display_name(right).to_ascii_lowercase();
            left_name.cmp(&right_name)
        }
    }
}

fn is_config_name(name: &str) -> bool {
    name.ends_with(".conf")
        || name.ends_with(".toml")
        || name.ends_with(".yaml")
        || name.ends_with(".yml")
        || name.ends_with(".json")
        || name.ends_with(".ini")
}

fn is_script_name(name: &str) -> bool {
    name.ends_with(".sh")
        || name.ends_with(".bash")
        || name.ends_with(".py")
        || name.ends_with(".pl")
        || name.ends_with(".rb")
}

fn read_text_preview(path: &Path, max_lines: usize) -> Option<String> {
    let raw = fs::read(path).ok()?;
    let has_nul = raw.iter().take(4096).any(|byte| *byte == 0);
    if has_nul {
        return None;
    }
    let text = String::from_utf8(raw).ok()?;
    let mut lines = Vec::new();
    for line in text.lines().take(max_lines) {
        lines.push(line.to_string());
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}
