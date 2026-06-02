// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! Read-only project view model shared by the CLI, TUI, and future ProGit Remote.

use crate::issue::Issue;
use crate::storage;
use anyhow::{bail, Context, Result};
use kdl::{KdlDocument, KdlNode};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Renderable repository-owned wiki surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWikiView {
    /// Root page path declared by `.project/wiki/manifest.kdl`.
    pub root: PathBuf,
    /// Markdown pages declared in manifest order.
    pub pages: Vec<ProjectWikiPage>,
}

/// One repository-owned Markdown wiki page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWikiPage {
    /// Stable logical page name.
    pub name: String,
    /// Human-facing title from the manifest.
    pub title: String,
    /// Project-relative Markdown path.
    pub path: PathBuf,
    /// True when the page is required by the manifest.
    pub required: bool,
    /// Raw Markdown content. Renderers decide how to display it.
    pub content: String,
}

/// Read-only issue surface for `.project/issues/*.json`.
#[derive(Debug, Clone)]
pub struct ProjectIssuesView {
    /// Issue files sorted by project-relative path.
    pub issues: Vec<ProjectIssueEntry>,
}

/// One issue loaded from a repository-owned issue file.
#[derive(Debug, Clone)]
pub struct ProjectIssueEntry {
    /// Project-relative issue JSON path.
    pub path: PathBuf,
    /// Parsed issue payload.
    pub issue: Issue,
}

/// Load `.project/wiki/manifest.kdl` and the Markdown pages it declares.
pub fn load_project_wiki(root: &Path) -> Result<ProjectWikiView> {
    let manifest_path = root
        .join(storage::paths::PROJECT_DIR)
        .join("wiki/manifest.kdl");
    let manifest_rel = rel_path(root, &manifest_path);
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_rel.display()))?;
    let doc = content
        .parse::<KdlDocument>()
        .with_context(|| format!("invalid KDL in {}", manifest_rel.display()))?;

    let wiki_nodes: Vec<&KdlNode> = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "wiki")
        .collect();
    if wiki_nodes.len() != 1 {
        bail!(
            "{} must contain exactly one top-level wiki node",
            manifest_rel.display()
        );
    }

    let manifest = wiki_nodes[0];
    let version = required_i64_child(manifest, "version", &manifest_rel)?;
    if version != 0 {
        bail!("wiki.version must be 0 for wiki v0");
    }

    let root_page = required_path_child(manifest, "root", &manifest_rel)?;
    validate_wiki_markdown_path(root, &root_page, "wiki.root")?;

    let mut pages = Vec::new();
    let mut page_names = HashSet::new();
    let mut page_paths = HashSet::new();
    for page_node in child_nodes(manifest, "page") {
        let page = load_wiki_page(root, page_node, &manifest_rel, &mut page_names)?;
        page_paths.insert(page.path.clone());
        pages.push(page);
    }

    if pages.is_empty() {
        bail!("{} must declare at least one page", manifest_rel.display());
    }

    if !page_paths.contains(&root_page) {
        bail!(
            "wiki.root `{}` must be declared by a page path",
            root_page.display()
        );
    }

    Ok(ProjectWikiView {
        root: root_page,
        pages,
    })
}

/// Load repository-owned issue files from `.project/issues/*.json`.
pub fn load_project_issues(root: &Path) -> Result<ProjectIssuesView> {
    let issues_dir = root.join(storage::paths::issues_dir());
    let issues_rel = rel_path(root, &issues_dir);
    if !issues_dir.is_dir() {
        bail!("{} must be a directory", issues_rel.display());
    }

    let mut issue_paths = Vec::new();
    for entry in fs::read_dir(&issues_dir)
        .with_context(|| format!("failed to read {}", issues_rel.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let file_type = entry.file_type().with_context(|| {
                format!("failed to inspect {}", rel_path(root, &path).display())
            })?;
            if !file_type.is_file() {
                bail!(
                    "{} must be a regular issue JSON file",
                    rel_path(root, &path).display()
                );
            }
            issue_paths.push(path);
        }
    }
    issue_paths.sort();

    let mut issues = Vec::with_capacity(issue_paths.len());
    for path in issue_paths {
        let rel = rel_path(root, &path);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", rel.display()))?;
        let issue = serde_json::from_str::<Issue>(&content)
            .with_context(|| format!("invalid issue JSON in {}", rel.display()))?;
        issues.push(ProjectIssueEntry { path: rel, issue });
    }

    Ok(ProjectIssuesView { issues })
}

fn load_wiki_page(
    root: &Path,
    page: &KdlNode,
    manifest_rel: &Path,
    page_names: &mut HashSet<String>,
) -> Result<ProjectWikiPage> {
    let Some(name) = positional_string_arg(page) else {
        bail!(
            "wiki page entries in {} must include a page name",
            manifest_rel.display()
        );
    };
    if !is_valid_wiki_page_name(&name) {
        bail!("wiki page name `{name}` contains invalid characters");
    }
    if !page_names.insert(name.clone()) {
        bail!("duplicate wiki page declaration `{name}`");
    }

    let title = required_string_child(page, "title", manifest_rel)?;
    if title.trim().is_empty() {
        bail!("wiki page `{name}` title must not be empty");
    }

    let path = required_path_child(page, "path", manifest_rel)?;
    validate_wiki_markdown_path(root, &path, &format!("wiki page `{name}` path"))?;

    let required = required_bool_child(page, "required", manifest_rel)?;
    let content = fs::read_to_string(root.join(&path))
        .with_context(|| format!("failed to read {}", path.display()))?;

    Ok(ProjectWikiPage {
        name,
        title,
        path,
        required,
        content,
    })
}

fn required_string_child(node: &KdlNode, name: &str, rel: &Path) -> Result<String> {
    child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_string())
        .map(ToString::to_string)
        .with_context(|| format!("{name} must be a string in {}", rel.display()))
}

fn required_path_child(node: &KdlNode, name: &str, rel: &Path) -> Result<PathBuf> {
    required_string_child(node, name, rel).map(PathBuf::from)
}

fn required_i64_child(node: &KdlNode, name: &str, rel: &Path) -> Result<i64> {
    child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_i64())
        .with_context(|| format!("{name} must be an integer in {}", rel.display()))
}

fn required_bool_child(node: &KdlNode, name: &str, rel: &Path) -> Result<bool> {
    child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_bool())
        .with_context(|| format!("{name} must be a boolean in {}", rel.display()))
}

fn child_node<'a>(node: &'a KdlNode, name: &str) -> Option<&'a KdlNode> {
    node.children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .find(|child| child.name().value() == name)
}

fn child_nodes<'a>(node: &'a KdlNode, name: &str) -> Vec<&'a KdlNode> {
    node.children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .filter(move |child| child.name().value() == name)
        .collect()
}

fn positional_string_arg(node: &KdlNode) -> Option<String> {
    node.entries()
        .iter()
        .find(|entry| entry.name().is_none())
        .and_then(|entry| entry.value().as_string())
        .map(ToString::to_string)
}

fn validate_wiki_markdown_path(root: &Path, path: &Path, label: &str) -> Result<()> {
    if !is_safe_project_wiki_path(path) {
        bail!("{label} must be relative and stay under .project/wiki/");
    }

    if !path
        .extension()
        .is_some_and(|ext| ext == "md" || ext == "markdown")
    {
        bail!("{label} must point to a Markdown file");
    }

    if !root.join(path).is_file() {
        bail!(
            "{label} `{}` must point to an existing file",
            path.display()
        );
    }

    Ok(())
}

fn is_safe_project_wiki_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(first)) if first == ".project" => {}
        _ => return false,
    }
    match components.next() {
        Some(Component::Normal(second)) if second == "wiki" => {}
        _ => return false,
    }

    let mut saw_page_path = false;
    for component in components {
        match component {
            Component::Normal(_) => saw_page_path = true,
            _ => return false,
        }
    }

    saw_page_path
}

fn is_valid_wiki_page_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn rel_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn init_project(root: &Path) {
        fs::create_dir_all(root.join(".project/issues")).unwrap();
        fs::create_dir_all(root.join(".project/wiki")).unwrap();
    }

    fn write_wiki(root: &Path, manifest: &str) {
        fs::write(root.join(".project/wiki/manifest.kdl"), manifest).unwrap();
        fs::write(
            root.join(".project/wiki/index.md"),
            "# Index\n\nRoot page.\n",
        )
        .unwrap();
    }

    fn valid_wiki_manifest() -> &'static str {
        r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "index" {
        title "Index"
        path ".project/wiki/index.md"
        required true
    }
}
"#
    }

    #[test]
    fn load_wiki_reads_manifest_pages_and_markdown() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_wiki(dir.path(), valid_wiki_manifest());

        let view = load_project_wiki(dir.path()).unwrap();

        assert_eq!(view.root, PathBuf::from(".project/wiki/index.md"));
        assert_eq!(view.pages.len(), 1);
        assert_eq!(view.pages[0].name, "index");
        assert!(view.pages[0].content.contains("Root page"));
    }

    #[test]
    fn load_wiki_rejects_root_not_declared_by_page() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        fs::write(root_path(dir.path(), "root.md"), "# Root\n").unwrap();
        write_wiki(
            dir.path(),
            r#"
wiki {
    version 0
    root ".project/wiki/root.md"

    page "index" {
        title "Index"
        path ".project/wiki/index.md"
        required true
    }
}
"#,
        );

        let err = load_project_wiki(dir.path()).unwrap_err().to_string();

        assert!(err.contains("must be declared by a page path"));
    }

    #[test]
    fn load_issues_reads_sorted_issue_files() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        let alpha = Issue::new("Alpha");
        let beta = Issue::new("Beta");
        fs::write(
            dir.path().join(".project/issues/b.json"),
            serde_json::to_string(&beta).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.path().join(".project/issues/a.json"),
            serde_json::to_string(&alpha).unwrap(),
        )
        .unwrap();

        let view = load_project_issues(dir.path()).unwrap();

        assert_eq!(view.issues.len(), 2);
        assert_eq!(view.issues[0].issue.title, "Alpha");
        assert_eq!(view.issues[1].issue.title, "Beta");
    }

    #[test]
    fn load_issues_rejects_malformed_json() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        fs::write(dir.path().join(".project/issues/broken.json"), "{ nope").unwrap();

        let err = load_project_issues(dir.path()).unwrap_err().to_string();

        assert!(err.contains("invalid issue JSON"));
    }

    #[cfg(unix)]
    #[test]
    fn load_issues_rejects_symlinked_issue_files() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        init_project(dir.path());
        let outside = dir.path().join("outside.json");
        fs::write(
            &outside,
            serde_json::to_string(&Issue::new("Outside")).unwrap(),
        )
        .unwrap();
        symlink(&outside, dir.path().join(".project/issues/link.json")).unwrap();

        let err = load_project_issues(dir.path()).unwrap_err().to_string();

        assert!(err.contains("regular issue JSON file"));
    }

    fn root_path(root: &Path, name: &str) -> PathBuf {
        root.join(".project/wiki").join(name)
    }
}
