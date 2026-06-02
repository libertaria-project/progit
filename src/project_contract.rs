//! Project contract validation for repository-owned `.project/` truth.

use crate::issue::Issue;
use crate::storage;
use anyhow::{Context, Result};
use kdl::{KdlDocument, KdlNode};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyMode {
    Advisory,
    Enforced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IssuePolicy {
    require_uuid: bool,
    allow_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectPolicy {
    mode: PolicyMode,
    issues: IssuePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedSchemaEntry {
    name: String,
    owner: String,
    required: bool,
}

const REQUIRED_CORE_SCHEMAS: &[&str] = &["progit.issue", "progit.policy", "progit.plugins"];

/// A validation message tied to an optional project-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectValidationMessage {
    /// Project-relative path related to the message.
    pub path: Option<PathBuf>,
    /// Human-readable validation message.
    pub message: String,
}

/// Result of validating a ProGit project contract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectValidationReport {
    /// Number of checks that passed.
    pub checks_passed: usize,
    /// Non-fatal findings.
    pub warnings: Vec<ProjectValidationMessage>,
    /// Fatal contract violations.
    pub errors: Vec<ProjectValidationMessage>,
}

impl ProjectValidationReport {
    /// Returns true when the project has no fatal contract violations.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    fn pass(&mut self) {
        self.checks_passed += 1;
    }

    fn warn(&mut self, path: Option<PathBuf>, message: impl Into<String>) {
        self.warnings.push(ProjectValidationMessage {
            path,
            message: message.into(),
        });
    }

    fn error(&mut self, path: Option<PathBuf>, message: impl Into<String>) {
        self.errors.push(ProjectValidationMessage {
            path,
            message: message.into(),
        });
    }
}

/// Validate the v0 ProGit project contract rooted at `root`.
pub fn validate_project(root: &Path) -> Result<ProjectValidationReport> {
    let mut report = ProjectValidationReport::default();
    let project_dir = root.join(storage::paths::PROJECT_DIR);

    if !project_dir.exists() {
        report.error(
            Some(PathBuf::from(storage::paths::PROJECT_DIR)),
            ".project/ is required",
        );
        return Ok(report);
    }

    if !project_dir.is_dir() {
        report.error(
            Some(PathBuf::from(storage::paths::PROJECT_DIR)),
            ".project must be a directory",
        );
        return Ok(report);
    }
    report.pass();

    validate_config(root, &mut report)?;
    let policy = validate_policy(root, &mut report, &project_dir.join("policy.kdl"))?;
    validate_plugins(root, &mut report, &project_dir.join("plugins.kdl"))?;
    validate_schemas(root, &mut report, &project_dir.join("schemas"))?;
    validate_wiki(root, &mut report, &project_dir.join("wiki"))?;
    validate_issues(root, &mut report, policy.as_ref())?;

    Ok(report)
}

fn validate_config(root: &Path, report: &mut ProjectValidationReport) -> Result<()> {
    let config_path = root.join(storage::paths::config_file());
    let rel = rel_path(root, &config_path);

    if !config_path.exists() {
        report.error(Some(rel), ".project/config.kdl is required");
        return Ok(());
    }

    if !config_path.is_file() {
        report.error(Some(rel), ".project/config.kdl must be a file");
        return Ok(());
    }

    match storage::config::load_config(&config_path) {
        Ok(_) => report.pass(),
        Err(err) => report.error(Some(rel), format!("invalid config KDL: {err}")),
    }

    Ok(())
}

fn validate_policy(
    root: &Path,
    report: &mut ProjectValidationReport,
    path: &Path,
) -> Result<Option<ProjectPolicy>> {
    let rel = PathBuf::from(".project/policy.kdl");

    if !path.exists() {
        report.warn(Some(rel), "policy contract is not defined yet");
        return Ok(None);
    }

    if !path.is_file() {
        report.error(Some(rel), ".project/policy.kdl must be a file");
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", rel_path(root, path).display()))?;

    let doc = match content.parse::<KdlDocument>() {
        Ok(doc) => doc,
        Err(err) => {
            report.error(Some(rel), format!("invalid KDL: {err}"));
            return Ok(None);
        }
    };

    let errors_before = report.errors.len();
    let policy_nodes: Vec<&KdlNode> = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "policy")
        .collect();

    if policy_nodes.len() != 1 {
        report.error(
            Some(rel.clone()),
            ".project/policy.kdl must contain exactly one top-level policy node",
        );
        return Ok(None);
    }

    let policy_node = policy_nodes[0];
    let version = required_i64_child(report, &rel, policy_node, "version");
    if let Some(version) = version {
        if version != 0 {
            report.error(Some(rel.clone()), "policy.version must be 0 for policy v0");
        }
    }

    let mode = required_policy_mode(report, &rel, policy_node, "mode");
    let issues = validate_issue_policy(report, &rel, policy_node);
    validate_plugin_policy(report, &rel, policy_node);
    validate_sync_policy(report, &rel, policy_node);
    validate_audit_policy(report, &rel, policy_node);

    if report.errors.len() == errors_before {
        report.pass();
        Ok(Some(ProjectPolicy {
            mode: mode.unwrap_or(PolicyMode::Advisory),
            issues: issues.unwrap_or(IssuePolicy {
                require_uuid: false,
                allow_empty: true,
            }),
        }))
    } else {
        Ok(None)
    }
}

fn validate_issue_policy(
    report: &mut ProjectValidationReport,
    rel: &Path,
    policy_node: &KdlNode,
) -> Option<IssuePolicy> {
    let issues = required_section(report, rel, policy_node, "issues")?;
    let require_uuid = required_bool_child(report, rel, issues, "require-uuid");
    let require_title = required_bool_child(report, rel, issues, "require-title");
    let allow_empty = required_bool_child(report, rel, issues, "allow-empty");

    if require_title == Some(false) {
        report.error(
            Some(rel.to_path_buf()),
            "issues.require-title must be true in policy v0",
        );
    }

    match (require_uuid, allow_empty) {
        (Some(require_uuid), Some(allow_empty)) => Some(IssuePolicy {
            require_uuid,
            allow_empty,
        }),
        _ => None,
    }
}

fn validate_plugin_policy(report: &mut ProjectValidationReport, rel: &Path, policy_node: &KdlNode) {
    if let Some(plugins) = required_section(report, rel, policy_node, "plugins") {
        let _ = required_policy_mode(report, rel, plugins, "trust-policy");
    }
}

fn validate_sync_policy(report: &mut ProjectValidationReport, rel: &Path, policy_node: &KdlNode) {
    if let Some(sync) = required_section(report, rel, policy_node, "sync") {
        let _ = required_bool_child(report, rel, sync, "validate-before-push");
        let _ = required_bool_child(report, rel, sync, "validate-after-pull");
    }
}

fn validate_audit_policy(report: &mut ProjectValidationReport, rel: &Path, policy_node: &KdlNode) {
    if let Some(audit) = required_section(report, rel, policy_node, "audit") {
        let _ = required_bool_child(report, rel, audit, "enabled");
        if let Some(path) = required_string_child(report, rel, audit, "path") {
            if !is_safe_project_path(&path) {
                report.error(
                    Some(rel.to_path_buf()),
                    "audit.path must be relative and stay under .project/",
                );
            }
        }
    }
}

fn validate_plugins(root: &Path, report: &mut ProjectValidationReport, path: &Path) -> Result<()> {
    let rel = PathBuf::from(".project/plugins.kdl");

    if !path.exists() {
        report.warn(Some(rel), "plugin trust contract is not defined yet");
        return Ok(());
    }

    if !path.is_file() {
        report.error(Some(rel), ".project/plugins.kdl must be a file");
        return Ok(());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", rel_path(root, path).display()))?;

    let doc = match content.parse::<KdlDocument>() {
        Ok(doc) => doc,
        Err(err) => {
            report.error(Some(rel), format!("invalid KDL: {err}"));
            return Ok(());
        }
    };

    let errors_before = report.errors.len();
    let plugin_manifests: Vec<&KdlNode> = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "plugins")
        .collect();

    if plugin_manifests.len() != 1 {
        report.error(
            Some(rel.clone()),
            ".project/plugins.kdl must contain exactly one top-level plugins node",
        );
        return Ok(());
    }

    let manifest = plugin_manifests[0];
    let version = required_i64_child(report, &rel, manifest, "version");
    if let Some(version) = version {
        if version != 0 {
            report.error(
                Some(rel.clone()),
                "plugins.version must be 0 for plugins v0",
            );
        }
    }

    if let Some(registry_url) = optional_string_child(report, &rel, manifest, "registry-url") {
        if !is_http_url(&registry_url) {
            report.error(
                Some(rel.clone()),
                "plugins.registry-url must use http:// or https://",
            );
        }
    }

    let plugin_nodes: Vec<&KdlNode> = manifest
        .children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .filter(|node| node.name().value() == "plugin")
        .collect();

    if plugin_nodes.is_empty() {
        report.warn(
            Some(rel.clone()),
            ".project/plugins.kdl declares no plugins",
        );
    }

    let mut names = HashSet::new();
    for plugin in plugin_nodes {
        validate_plugin_entry(report, &rel, plugin, &mut names);
    }

    if report.errors.len() == errors_before {
        report.pass();
    }

    Ok(())
}

fn validate_plugin_entry(
    report: &mut ProjectValidationReport,
    rel: &Path,
    plugin: &KdlNode,
    names: &mut HashSet<String>,
) {
    let Some(name) = positional_string_arg(plugin) else {
        report.error(
            Some(rel.to_path_buf()),
            "plugin entries must include a plugin name",
        );
        return;
    };

    if !is_valid_plugin_name(&name) {
        report.error(
            Some(rel.to_path_buf()),
            format!("plugin name `{name}` contains invalid characters"),
        );
    }

    if !names.insert(name.clone()) {
        report.error(
            Some(rel.to_path_buf()),
            format!("duplicate plugin declaration `{name}`"),
        );
    }

    let source = required_string_child(report, rel, plugin, "source");
    if let Some(source) = source.as_deref() {
        if !matches!(source, "registry" | "git" | "local") {
            report.error(
                Some(rel.to_path_buf()),
                format!("plugin `{name}` source must be registry, git, or local"),
            );
        }

        if matches!(source, "registry" | "git")
            && optional_string_child(report, rel, plugin, "version").is_none()
        {
            report.warn(
                Some(rel.to_path_buf()),
                format!("plugin `{name}` should pin a version for {source} source"),
            );
        }
    }

    let _ = required_bool_child(report, rel, plugin, "required");

    if let Some(checksum) = optional_string_child(report, rel, plugin, "checksum") {
        if !checksum.starts_with("sha256:") && !checksum.starts_with("blake3:") {
            report.error(
                Some(rel.to_path_buf()),
                format!("plugin `{name}` checksum must start with sha256: or blake3:"),
            );
        }
    }

    validate_capability_grants(report, rel, plugin, &name);
}

fn validate_capability_grants(
    report: &mut ProjectValidationReport,
    rel: &Path,
    plugin: &KdlNode,
    plugin_name: &str,
) {
    let Some(capabilities) = required_section(report, rel, plugin, "capabilities") else {
        return;
    };

    let capability_nodes: Vec<&KdlNode> = capabilities
        .children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .filter(|node| node.name().value() == "capability")
        .collect();

    if capability_nodes.is_empty() {
        report.error(
            Some(rel.to_path_buf()),
            format!("plugin `{plugin_name}` must declare at least one capability"),
        );
        return;
    }

    let mut seen = HashSet::new();
    for capability_node in capability_nodes {
        let Some(capability) = positional_string_arg(capability_node) else {
            report.error(
                Some(rel.to_path_buf()),
                format!("plugin `{plugin_name}` capability entries must be strings"),
            );
            continue;
        };

        if !is_allowed_capability(&capability) {
            report.error(
                Some(rel.to_path_buf()),
                format!("plugin `{plugin_name}` declares unknown capability `{capability}`"),
            );
        }

        if !seen.insert(capability.clone()) {
            report.warn(
                Some(rel.to_path_buf()),
                format!("plugin `{plugin_name}` repeats capability `{capability}`"),
            );
        }
    }
}

fn validate_schemas(
    root: &Path,
    report: &mut ProjectValidationReport,
    schemas_dir: &Path,
) -> Result<()> {
    let rel_dir = PathBuf::from(".project/schemas");

    if !schemas_dir.exists() {
        report.warn(
            Some(rel_dir),
            "project-local schemas directory is not present",
        );
        return Ok(());
    }

    if !schemas_dir.is_dir() {
        report.error(
            Some(rel_path(root, schemas_dir)),
            ".project/schemas must be a directory",
        );
        return Ok(());
    }
    report.pass();

    let manifest_path = schemas_dir.join("manifest.kdl");
    let rel = PathBuf::from(".project/schemas/manifest.kdl");

    if !manifest_path.exists() {
        report.error(
            Some(rel),
            ".project/schemas/manifest.kdl is required when .project/schemas/ exists",
        );
        return Ok(());
    }

    if !manifest_path.is_file() {
        report.error(Some(rel), ".project/schemas/manifest.kdl must be a file");
        return Ok(());
    }

    let content = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "Failed to read {}",
            rel_path(root, &manifest_path).display()
        )
    })?;

    let doc = match content.parse::<KdlDocument>() {
        Ok(doc) => doc,
        Err(err) => {
            report.error(Some(rel), format!("invalid KDL: {err}"));
            return Ok(());
        }
    };

    let errors_before = report.errors.len();
    let schema_manifests: Vec<&KdlNode> = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "schemas")
        .collect();

    if schema_manifests.len() != 1 {
        report.error(
            Some(rel.clone()),
            ".project/schemas/manifest.kdl must contain exactly one top-level schemas node",
        );
        return Ok(());
    }

    let manifest = schema_manifests[0];
    let version = required_i64_child(report, &rel, manifest, "version");
    if let Some(version) = version {
        if version != 0 {
            report.error(
                Some(rel.clone()),
                "schemas.version must be 0 for schemas v0",
            );
        }
    }

    let schema_nodes: Vec<&KdlNode> = manifest
        .children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .filter(|node| node.name().value() == "schema")
        .collect();

    let mut names = HashSet::new();
    let mut core_schemas = HashMap::new();
    for schema in schema_nodes {
        if let Some(entry) = validate_schema_entry(root, report, &rel, schema, &mut names) {
            if entry.name.starts_with("progit.") && entry.owner != "core" {
                report.error(
                    Some(rel.clone()),
                    format!("schema `{}` uses reserved progit.* namespace", entry.name),
                );
            }

            if REQUIRED_CORE_SCHEMAS.contains(&entry.name.as_str()) {
                core_schemas.insert(entry.name.clone(), entry);
            }
        }
    }

    for schema_name in REQUIRED_CORE_SCHEMAS {
        match core_schemas.get(*schema_name) {
            Some(entry) => {
                if entry.owner != "core" {
                    report.error(
                        Some(rel.clone()),
                        format!("required core schema `{schema_name}` must be owned by core"),
                    );
                }
                if !entry.required {
                    report.error(
                        Some(rel.clone()),
                        format!("required core schema `{schema_name}` must set required true"),
                    );
                }
            }
            None => report.error(
                Some(rel.clone()),
                format!("required core schema `{schema_name}` is missing"),
            ),
        }
    }

    if report.errors.len() == errors_before {
        report.pass();
    }

    Ok(())
}

fn validate_schema_entry(
    root: &Path,
    report: &mut ProjectValidationReport,
    rel: &Path,
    schema: &KdlNode,
    names: &mut HashSet<String>,
) -> Option<ValidatedSchemaEntry> {
    let Some(name) = positional_string_arg(schema) else {
        report.error(
            Some(rel.to_path_buf()),
            "schema entries must include a schema name",
        );
        return None;
    };

    if !is_valid_schema_name(&name) {
        report.error(
            Some(rel.to_path_buf()),
            format!("schema name `{name}` contains invalid characters or lacks a namespace"),
        );
    }

    let duplicate_name = !names.insert(name.clone());
    if duplicate_name {
        report.error(
            Some(rel.to_path_buf()),
            format!("duplicate schema declaration `{name}`"),
        );
    }

    let owner = required_string_child(report, rel, schema, "owner");
    let owner = match owner.as_deref() {
        Some("core") => "core",
        Some("plugin") => "plugin",
        Some(_) => {
            report.error(
                Some(rel.to_path_buf()),
                format!("schema `{name}` owner must be core or plugin"),
            );
            return None;
        }
        None => return None,
    };

    if owner == "plugin" {
        match required_string_child(report, rel, schema, "plugin") {
            Some(plugin_name) if is_valid_plugin_name(&plugin_name) => {}
            Some(plugin_name) => report.error(
                Some(rel.to_path_buf()),
                format!("plugin-owned schema `{name}` has invalid plugin `{plugin_name}`"),
            ),
            None => report.error(
                Some(rel.to_path_buf()),
                format!("plugin-owned schema `{name}` must declare plugin"),
            ),
        }
    }

    if let Some(path) = required_string_child(report, rel, schema, "path") {
        if !is_safe_project_schemas_path(&path) {
            report.error(
                Some(rel.to_path_buf()),
                format!("schema `{name}` path must be relative and stay under .project/schemas/"),
            );
        } else if !root.join(&path).is_file() {
            report.error(
                Some(rel.to_path_buf()),
                format!("schema `{name}` path `{path}` must point to an existing file"),
            );
        }
    }

    let required = required_bool_child(report, rel, schema, "required")?;
    if duplicate_name {
        return None;
    }

    Some(ValidatedSchemaEntry {
        name,
        owner: owner.to_string(),
        required,
    })
}

fn validate_wiki(root: &Path, report: &mut ProjectValidationReport, wiki_dir: &Path) -> Result<()> {
    let rel_dir = PathBuf::from(".project/wiki");

    if !wiki_dir.exists() {
        report.warn(Some(rel_dir), "project wiki directory is not present");
        return Ok(());
    }

    if !wiki_dir.is_dir() {
        report.error(
            Some(rel_path(root, wiki_dir)),
            ".project/wiki must be a directory",
        );
        return Ok(());
    }
    report.pass();

    let manifest_path = wiki_dir.join("manifest.kdl");
    let rel = PathBuf::from(".project/wiki/manifest.kdl");

    if !manifest_path.exists() {
        report.error(
            Some(rel),
            ".project/wiki/manifest.kdl is required when .project/wiki/ exists",
        );
        return Ok(());
    }

    if !manifest_path.is_file() {
        report.error(Some(rel), ".project/wiki/manifest.kdl must be a file");
        return Ok(());
    }

    let content = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "Failed to read {}",
            rel_path(root, &manifest_path).display()
        )
    })?;

    let doc = match content.parse::<KdlDocument>() {
        Ok(doc) => doc,
        Err(err) => {
            report.error(Some(rel), format!("invalid KDL: {err}"));
            return Ok(());
        }
    };

    let errors_before = report.errors.len();
    let wiki_manifests: Vec<&KdlNode> = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "wiki")
        .collect();

    if wiki_manifests.len() != 1 {
        report.error(
            Some(rel.clone()),
            ".project/wiki/manifest.kdl must contain exactly one top-level wiki node",
        );
        return Ok(());
    }

    let manifest = wiki_manifests[0];
    let version = required_i64_child(report, &rel, manifest, "version");
    if let Some(version) = version {
        if version != 0 {
            report.error(Some(rel.clone()), "wiki.version must be 0 for wiki v0");
        }
    }

    let root_page = required_string_child(report, &rel, manifest, "root");
    if let Some(root_page) = root_page.as_deref() {
        validate_wiki_markdown_path(root, report, &rel, "wiki.root", root_page);
    }

    let page_nodes: Vec<&KdlNode> = manifest
        .children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .filter(|node| node.name().value() == "page")
        .collect();

    if page_nodes.is_empty() {
        report.error(
            Some(rel.clone()),
            ".project/wiki/manifest.kdl must declare at least one page",
        );
    }

    let mut names = HashSet::new();
    let mut page_paths = HashSet::new();
    for page in page_nodes {
        if let Some(path) = validate_wiki_page(root, report, &rel, page, &mut names) {
            page_paths.insert(path);
        }
    }

    if let Some(root_page) = root_page {
        if !page_paths.contains(&root_page) {
            report.error(
                Some(rel.clone()),
                format!("wiki.root `{root_page}` must be declared by a page path"),
            );
        }
    }

    if report.errors.len() == errors_before {
        report.pass();
    }

    Ok(())
}

fn validate_wiki_page(
    root: &Path,
    report: &mut ProjectValidationReport,
    rel: &Path,
    page: &KdlNode,
    names: &mut HashSet<String>,
) -> Option<String> {
    let Some(name) = positional_string_arg(page) else {
        report.error(
            Some(rel.to_path_buf()),
            "wiki page entries must include a page name",
        );
        return None;
    };

    if !is_valid_wiki_page_name(&name) {
        report.error(
            Some(rel.to_path_buf()),
            format!("wiki page name `{name}` contains invalid characters"),
        );
    }

    let duplicate_name = !names.insert(name.clone());
    if duplicate_name {
        report.error(
            Some(rel.to_path_buf()),
            format!("duplicate wiki page declaration `{name}`"),
        );
    }

    match required_string_child(report, rel, page, "title") {
        Some(title) if title.trim().is_empty() => {
            report.error(
                Some(rel.to_path_buf()),
                format!("wiki page `{name}` title must not be empty"),
            );
        }
        Some(_) => {}
        None => {}
    }

    let path = required_string_child(report, rel, page, "path");
    if let Some(path) = path.as_deref() {
        validate_wiki_markdown_path(root, report, rel, &format!("wiki page `{name}` path"), path);
    }

    let _ = required_bool_child(report, rel, page, "required");

    if duplicate_name {
        None
    } else {
        path
    }
}

fn validate_wiki_markdown_path(
    root: &Path,
    report: &mut ProjectValidationReport,
    rel: &Path,
    label: &str,
    path: &str,
) {
    if !is_safe_project_wiki_path(path) {
        report.error(
            Some(rel.to_path_buf()),
            format!("{label} must be relative and stay under .project/wiki/"),
        );
        return;
    }

    let path_ref = Path::new(path);
    if !path_ref
        .extension()
        .is_some_and(|ext| ext == "md" || ext == "markdown")
    {
        report.error(Some(rel.to_path_buf()), format!("{label} must be Markdown"));
        return;
    }

    if !root.join(path_ref).is_file() {
        report.error(
            Some(rel.to_path_buf()),
            format!("{label} `{path}` must point to an existing file"),
        );
    }
}

fn validate_issues(
    root: &Path,
    report: &mut ProjectValidationReport,
    policy: Option<&ProjectPolicy>,
) -> Result<()> {
    let issues_dir = root.join(storage::paths::issues_dir());
    let rel = rel_path(root, &issues_dir);

    if !issues_dir.exists() {
        report.error(Some(rel), ".project/issues/ is required");
        return Ok(());
    }

    if !issues_dir.is_dir() {
        report.error(Some(rel), ".project/issues must be a directory");
        return Ok(());
    }
    report.pass();

    let mut issue_paths = Vec::new();
    for entry in fs::read_dir(&issues_dir)
        .with_context(|| format!("Failed to read {}", rel_path(root, &issues_dir).display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let file_type = entry.file_type().with_context(|| {
                format!("Failed to inspect {}", rel_path(root, &path).display())
            })?;
            if !file_type.is_file() {
                report.error(
                    Some(rel_path(root, &path)),
                    ".project/issues entries must be regular JSON files",
                );
                continue;
            }
            issue_paths.push(path);
        }
    }
    issue_paths.sort();

    if issue_paths.is_empty() {
        let message = ".project/issues contains no issue JSON files";
        if let Some(policy) = policy {
            if !policy.issues.allow_empty && policy.mode == PolicyMode::Enforced {
                report.error(Some(rel_path(root, &issues_dir)), message);
            } else if !policy.issues.allow_empty {
                report.warn(Some(rel_path(root, &issues_dir)), message);
            }
        } else {
            report.warn(Some(rel_path(root, &issues_dir)), message);
        }
        return Ok(());
    }

    let mut seen_ids: HashMap<String, PathBuf> = HashMap::new();

    for path in issue_paths {
        validate_issue_file(root, report, &path, &mut seen_ids, policy)?;
    }

    Ok(())
}

fn validate_issue_file(
    root: &Path,
    report: &mut ProjectValidationReport,
    path: &Path,
    seen_ids: &mut HashMap<String, PathBuf>,
    policy: Option<&ProjectPolicy>,
) -> Result<()> {
    let rel = rel_path(root, path);
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", rel.display()))?;

    let issue = match serde_json::from_str::<Issue>(&content) {
        Ok(issue) => issue,
        Err(err) => {
            report.error(Some(rel), format!("invalid issue JSON: {err}"));
            return Ok(());
        }
    };

    let id = issue.id.trim();
    if id.is_empty() {
        report.error(Some(rel.clone()), "issue id is required");
    } else if let Some(first_path) = seen_ids.insert(id.to_string(), rel.clone()) {
        report.error(
            Some(rel.clone()),
            format!(
                "duplicate issue id `{}`; first seen in {}",
                id,
                first_path.display()
            ),
        );
    } else if Uuid::parse_str(id).is_err() {
        if let Some(policy) = policy {
            if policy.issues.require_uuid && policy.mode == PolicyMode::Enforced {
                report.error(
                    Some(rel.clone()),
                    format!("issue id `{id}` must be a UUID by policy"),
                );
            } else if policy.issues.require_uuid {
                report.warn(
                    Some(rel.clone()),
                    format!("issue id `{id}` should be a UUID by policy"),
                );
            }
        } else {
            report.warn(
                Some(rel.clone()),
                format!("issue id `{id}` is not a UUID; future policy may require UUID ids"),
            );
        }
    }

    if issue.title.trim().is_empty() {
        report.error(Some(rel), "issue title is required");
    } else {
        report.pass();
    }

    Ok(())
}

fn rel_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn required_section<'a>(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &'a KdlNode,
    name: &str,
) -> Option<&'a KdlNode> {
    let section = child_node(node, name);
    if section.is_none() {
        report.error(
            Some(rel.to_path_buf()),
            format!("{name} section is required"),
        );
    }
    section
}

fn required_policy_mode(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &KdlNode,
    name: &str,
) -> Option<PolicyMode> {
    let value = required_string_child(report, rel, node, name)?;
    match value.as_str() {
        "advisory" => Some(PolicyMode::Advisory),
        "enforced" => Some(PolicyMode::Enforced),
        _ => {
            report.error(
                Some(rel.to_path_buf()),
                format!("{name} must be advisory or enforced"),
            );
            None
        }
    }
}

fn required_string_child(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &KdlNode,
    name: &str,
) -> Option<String> {
    let value = child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_string())
        .map(|value| value.to_string());
    if value.is_none() {
        report.error(Some(rel.to_path_buf()), format!("{name} must be a string"));
    }
    value
}

fn optional_string_child(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &KdlNode,
    name: &str,
) -> Option<String> {
    let Some(child) = child_node(node, name) else {
        return None;
    };

    let value = child
        .entries()
        .first()
        .and_then(|entry| entry.value().as_string())
        .map(|value| value.to_string());

    if value.is_none() {
        report.error(Some(rel.to_path_buf()), format!("{name} must be a string"));
    }

    value
}

fn required_i64_child(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &KdlNode,
    name: &str,
) -> Option<i64> {
    let value = child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_i64());
    if value.is_none() {
        report.error(
            Some(rel.to_path_buf()),
            format!("{name} must be an integer"),
        );
    }
    value
}

fn required_bool_child(
    report: &mut ProjectValidationReport,
    rel: &Path,
    node: &KdlNode,
    name: &str,
) -> Option<bool> {
    let value = child_node(node, name)
        .and_then(|node| node.entries().first())
        .and_then(|entry| entry.value().as_bool());
    if value.is_none() {
        report.error(Some(rel.to_path_buf()), format!("{name} must be a boolean"));
    }
    value
}

fn child_node<'a>(node: &'a KdlNode, name: &str) -> Option<&'a KdlNode> {
    node.children()
        .map(|children| children.nodes())
        .unwrap_or(&[])
        .iter()
        .find(|child| child.name().value() == name)
}

fn is_safe_project_path(path: &str) -> bool {
    let path = Path::new(path);
    if path.is_absolute() {
        return false;
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(first)) if first == ".project" => {}
        _ => return false,
    }

    components.all(|component| matches!(component, Component::Normal(_)))
}

fn is_safe_project_schemas_path(path: &str) -> bool {
    is_safe_project_child_path(path, "schemas")
}

fn is_safe_project_wiki_path(path: &str) -> bool {
    is_safe_project_child_path(path, "wiki")
}

fn is_safe_project_child_path(path: &str, child_dir: &str) -> bool {
    let path = Path::new(path);
    if path.is_absolute() {
        return false;
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(first)) if first == ".project" => {}
        _ => return false,
    }
    match components.next() {
        Some(Component::Normal(second)) if second == child_dir => {}
        _ => return false,
    }

    let mut saw_child_path = false;
    for component in components {
        match component {
            Component::Normal(_) => saw_child_path = true,
            _ => return false,
        }
    }

    saw_child_path
}

fn positional_string_arg(node: &KdlNode) -> Option<String> {
    node.entries()
        .iter()
        .find(|entry| entry.name().is_none())
        .and_then(|entry| entry.value().as_string())
        .map(|value| value.to_string())
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn is_valid_plugin_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn is_valid_schema_name(name: &str) -> bool {
    !name.is_empty()
        && name.contains('.')
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn is_valid_wiki_page_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn is_allowed_capability(capability: &str) -> bool {
    matches!(
        capability,
        "project:read"
            | "project:write"
            | "issues:read"
            | "issues:write"
            | "reviews:read"
            | "reviews:write"
            | "sync:read"
            | "sync:write"
            | "git-hooks:read"
            | "git-hooks:write"
            | "policy:evaluate"
            | "identity:read"
            | "identity:map"
            | "reputation:read"
            | "audit:write"
            | "ui:panel"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn init_project(root: &Path) {
        let project_dir = root.join(".project");
        fs::create_dir_all(project_dir.join("issues")).unwrap();
        fs::write(
            project_dir.join("config.kdl"),
            r#"
config {
    sprint-duration-days 14
}
theme "nord"
"#,
        )
        .unwrap();
    }

    fn write_issue(root: &Path, filename: &str, issue: &Issue) {
        let content = serde_json::to_string_pretty(issue).unwrap();
        fs::write(root.join(".project").join("issues").join(filename), content).unwrap();
    }

    fn write_policy(root: &Path, mode: &str, require_uuid: bool, allow_empty: bool) {
        fs::write(
            root.join(".project").join("policy.kdl"),
            format!(
                r#"
policy {{
    version 0
    mode "{mode}"

    issues {{
        require-uuid {require_uuid}
        require-title true
        allow-empty {allow_empty}
    }}

    plugins {{
        trust-policy "advisory"
    }}

    sync {{
        validate-before-push true
        validate-after-pull true
    }}

    audit {{
        enabled false
        path ".project/audit/events.jsonl"
    }}
}}
"#
            ),
        )
        .unwrap();
    }

    fn write_plugins(root: &Path, body: &str) {
        fs::write(root.join(".project").join("plugins.kdl"), body).unwrap();
    }

    fn valid_plugins_manifest() -> &'static str {
        r#"
plugins {
    version 0
    registry-url "https://plugins.progit.dev"

    plugin "git-hooks" {
        source "registry"
        version ">=0.1.0"
        required false

        capabilities {
            capability "project:read"
            capability "git-hooks:write"
        }
    }
}
"#
    }

    fn write_schemas_manifest(root: &Path, body: &str) {
        let schemas_dir = root.join(".project").join("schemas");
        fs::create_dir_all(&schemas_dir).unwrap();
        fs::write(schemas_dir.join("manifest.kdl"), body).unwrap();
    }

    fn write_schema_descriptor(
        root: &Path,
        filename: &str,
        schema_name: &str,
        schema_format: &str,
    ) {
        fs::write(
            root.join(".project").join("schemas").join(filename),
            format!(
                r#"
schema "{schema_name}" {{
    version 0
    format "{schema_format}"
    owner "core"
    status "declared"
}}
"#
            ),
        )
        .unwrap();
    }

    fn valid_schemas_manifest() -> &'static str {
        r#"
schemas {
    version 0

    schema "progit.issue" {
        owner "core"
        path ".project/schemas/issue.v0.schema.kdl"
        required true
    }

    schema "progit.policy" {
        owner "core"
        path ".project/schemas/policy.v0.schema.kdl"
        required true
    }

    schema "progit.plugins" {
        owner "core"
        path ".project/schemas/plugins.v0.schema.kdl"
        required true
    }
}
"#
    }

    fn write_valid_schemas(root: &Path) {
        write_schemas_manifest(root, valid_schemas_manifest());
        write_schema_descriptor(root, "issue.v0.schema.kdl", "progit.issue", "json");
        write_schema_descriptor(root, "policy.v0.schema.kdl", "progit.policy", "kdl");
        write_schema_descriptor(root, "plugins.v0.schema.kdl", "progit.plugins", "kdl");
    }

    fn write_wiki_manifest(root: &Path, body: &str) {
        let wiki_dir = root.join(".project").join("wiki");
        fs::create_dir_all(&wiki_dir).unwrap();
        fs::write(wiki_dir.join("manifest.kdl"), body).unwrap();
    }

    fn write_wiki_page(root: &Path, filename: &str, title: &str) {
        fs::write(
            root.join(".project").join("wiki").join(filename),
            format!("# {title}\n\nProject-owned wiki page.\n"),
        )
        .unwrap();
    }

    fn valid_wiki_manifest() -> &'static str {
        r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "index" {
        title "ProGit"
        path ".project/wiki/index.md"
        required true
    }
}
"#
    }

    fn write_valid_wiki(root: &Path) {
        write_wiki_manifest(root, valid_wiki_manifest());
        write_wiki_page(root, "index.md", "ProGit");
    }

    #[test]
    fn valid_project_passes_with_v0_warnings() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(report.checks_passed >= 3);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("policy contract")));
    }

    #[test]
    fn missing_project_dir_fails() {
        let dir = tempdir().unwrap();

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains(".project/ is required")));
    }

    #[test]
    fn malformed_issue_json_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        fs::write(
            dir.path()
                .join(".project")
                .join("issues")
                .join("broken.json"),
            "{ nope",
        )
        .unwrap();

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("invalid issue JSON")));
    }

    #[test]
    fn duplicate_issue_ids_fail() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        let first = Issue::new("First");
        let mut second = Issue::new("Second");
        second.id = first.id.clone();
        write_issue(dir.path(), "first.json", &first);
        write_issue(dir.path(), "second.json", &second);

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("duplicate issue id")));
    }

    #[test]
    fn invalid_optional_kdl_fails_when_present() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        fs::write(dir.path().join(".project").join("policy.kdl"), "policy {").unwrap();

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("invalid KDL")));
    }

    #[test]
    fn valid_policy_schema_passes() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "advisory", true, true);
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("policy contract")));
    }

    #[test]
    fn invalid_policy_mode_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "strict", true, true);
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("mode must be advisory or enforced")));
    }

    #[test]
    fn enforced_policy_rejects_non_uuid_issue_ids() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "enforced", true, true);
        let mut issue = Issue::new("Legacy issue");
        issue.id = "legacy-issue-1".to_string();
        write_issue(dir.path(), "legacy.json", &issue);

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("must be a UUID by policy")));
    }

    #[test]
    fn advisory_policy_warns_on_non_uuid_issue_ids() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "advisory", true, true);
        let mut issue = Issue::new("Legacy issue");
        issue.id = "legacy-issue-1".to_string();
        write_issue(dir.path(), "legacy.json", &issue);

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("should be a UUID by policy")));
    }

    #[test]
    fn enforced_policy_rejects_empty_issue_directory_when_disallowed() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "enforced", true, false);

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("contains no issue JSON files")));
    }

    #[test]
    fn invalid_audit_path_fails_policy_validation() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        fs::write(
            dir.path().join(".project").join("policy.kdl"),
            r#"
policy {
    version 0
    mode "advisory"

    issues {
        require-uuid true
        require-title true
        allow-empty true
    }

    plugins {
        trust-policy "advisory"
    }

    sync {
        validate-before-push true
        validate-after-pull true
    }

    audit {
        enabled false
        path "../audit/events.jsonl"
    }
}
"#,
        )
        .unwrap();

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("audit.path must be relative")));
    }

    #[test]
    fn valid_plugins_manifest_passes() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "advisory", true, true);
        write_plugins(dir.path(), valid_plugins_manifest());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("plugin trust contract")));
    }

    #[test]
    fn duplicate_plugin_names_fail() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        write_plugins(
            dir.path(),
            r#"
plugins {
    version 0

    plugin "git-hooks" {
        source "registry"
        version ">=0.1.0"
        required false
        capabilities {
            capability "project:read"
        }
    }

    plugin "git-hooks" {
        source "registry"
        version ">=0.1.0"
        required false
        capabilities {
            capability "project:read"
        }
    }
}
"#,
        );

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("duplicate plugin declaration")));
    }

    #[test]
    fn unknown_plugin_capability_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        write_plugins(
            dir.path(),
            r#"
plugins {
    version 0

    plugin "identity-bridge" {
        source "registry"
        version ">=0.1.0"
        required true
        capabilities {
            capability "root:everything"
        }
    }
}
"#,
        );

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("unknown capability")));
    }

    #[test]
    fn invalid_plugin_registry_url_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        write_plugins(
            dir.path(),
            r#"
plugins {
    version 0
    registry-url "file:///tmp/plugins"
}
"#,
        );

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("registry-url must use")));
    }

    #[test]
    fn plugin_without_capabilities_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));
        write_plugins(
            dir.path(),
            r#"
plugins {
    version 0

    plugin "git-hooks" {
        source "registry"
        version ">=0.1.0"
        required false
    }
}
"#,
        );

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("capabilities section is required")));
    }

    #[test]
    fn valid_schemas_manifest_passes() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "advisory", true, true);
        write_plugins(dir.path(), valid_plugins_manifest());
        write_valid_schemas(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("schemas directory")));
    }

    #[test]
    fn schemas_directory_without_manifest_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        fs::create_dir_all(dir.path().join(".project").join("schemas")).unwrap();
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("manifest.kdl is required")));
    }

    #[test]
    fn duplicate_schema_names_fail() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_schemas_manifest(
            dir.path(),
            r#"
schemas {
    version 0

    schema "progit.issue" {
        owner "core"
        path ".project/schemas/issue.v0.schema.kdl"
        required true
    }

    schema "progit.policy" {
        owner "core"
        path ".project/schemas/policy.v0.schema.kdl"
        required true
    }

    schema "progit.plugins" {
        owner "core"
        path ".project/schemas/plugins.v0.schema.kdl"
        required true
    }

    schema "acme.widget" {
        owner "plugin"
        plugin "acme"
        path ".project/schemas/acme.widget.v0.schema.kdl"
        required false
    }

    schema "acme.widget" {
        owner "plugin"
        plugin "acme"
        path ".project/schemas/acme.widget.v0.schema.kdl"
        required false
    }
}
"#,
        );
        write_schema_descriptor(dir.path(), "issue.v0.schema.kdl", "progit.issue", "json");
        write_schema_descriptor(dir.path(), "policy.v0.schema.kdl", "progit.policy", "kdl");
        write_schema_descriptor(dir.path(), "plugins.v0.schema.kdl", "progit.plugins", "kdl");
        write_schema_descriptor(
            dir.path(),
            "acme.widget.v0.schema.kdl",
            "acme.widget",
            "kdl",
        );
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("duplicate schema declaration")));
    }

    #[test]
    fn schema_path_outside_schemas_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_schemas_manifest(
            dir.path(),
            r#"
schemas {
    version 0

    schema "progit.issue" {
        owner "core"
        path ".project/policy.kdl"
        required true
    }

    schema "progit.policy" {
        owner "core"
        path ".project/schemas/policy.v0.schema.kdl"
        required true
    }

    schema "progit.plugins" {
        owner "core"
        path ".project/schemas/plugins.v0.schema.kdl"
        required true
    }
}
"#,
        );
        write_schema_descriptor(dir.path(), "policy.v0.schema.kdl", "progit.policy", "kdl");
        write_schema_descriptor(dir.path(), "plugins.v0.schema.kdl", "progit.plugins", "kdl");
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("stay under .project/schemas")));
    }

    #[test]
    fn plugin_owned_schema_without_plugin_field_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_schemas_manifest(
            dir.path(),
            r#"
schemas {
    version 0

    schema "progit.issue" {
        owner "core"
        path ".project/schemas/issue.v0.schema.kdl"
        required true
    }

    schema "progit.policy" {
        owner "core"
        path ".project/schemas/policy.v0.schema.kdl"
        required true
    }

    schema "progit.plugins" {
        owner "core"
        path ".project/schemas/plugins.v0.schema.kdl"
        required true
    }

    schema "acme.widget" {
        owner "plugin"
        path ".project/schemas/acme.widget.v0.schema.kdl"
        required false
    }
}
"#,
        );
        write_schema_descriptor(dir.path(), "issue.v0.schema.kdl", "progit.issue", "json");
        write_schema_descriptor(dir.path(), "policy.v0.schema.kdl", "progit.policy", "kdl");
        write_schema_descriptor(dir.path(), "plugins.v0.schema.kdl", "progit.plugins", "kdl");
        write_schema_descriptor(
            dir.path(),
            "acme.widget.v0.schema.kdl",
            "acme.widget",
            "kdl",
        );
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("plugin-owned schema")));
    }

    #[test]
    fn missing_referenced_schema_file_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_schemas_manifest(dir.path(), valid_schemas_manifest());
        write_schema_descriptor(dir.path(), "policy.v0.schema.kdl", "progit.policy", "kdl");
        write_schema_descriptor(dir.path(), "plugins.v0.schema.kdl", "progit.plugins", "kdl");
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("must point to an existing file")));
    }

    #[test]
    fn valid_wiki_manifest_passes() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_policy(dir.path(), "advisory", true, true);
        write_plugins(dir.path(), valid_plugins_manifest());
        write_valid_schemas(dir.path());
        write_valid_wiki(dir.path());
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(report.is_valid(), "{:#?}", report.errors);
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("project wiki directory")));
    }

    #[test]
    fn wiki_directory_without_manifest_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        fs::create_dir_all(dir.path().join(".project").join("wiki")).unwrap();
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("wiki/manifest.kdl is required")));
    }

    #[test]
    fn duplicate_wiki_pages_fail() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_wiki_manifest(
            dir.path(),
            r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "index" {
        title "ProGit"
        path ".project/wiki/index.md"
        required true
    }

    page "index" {
        title "Duplicate"
        path ".project/wiki/index.md"
        required false
    }
}
"#,
        );
        write_wiki_page(dir.path(), "index.md", "ProGit");
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("duplicate wiki page declaration")));
    }

    #[test]
    fn wiki_page_path_outside_wiki_fails() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_wiki_manifest(
            dir.path(),
            r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "index" {
        title "ProGit"
        path ".project/wiki/index.md"
        required true
    }

    page "readme" {
        title "README"
        path ".project/README.md"
        required false
    }
}
"#,
        );
        write_wiki_page(dir.path(), "index.md", "ProGit");
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("stay under .project/wiki")));
    }

    #[test]
    fn wiki_root_must_be_declared_by_page_path() {
        let dir = tempdir().unwrap();
        init_project(dir.path());
        write_wiki_manifest(
            dir.path(),
            r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "other" {
        title "Other"
        path ".project/wiki/other.md"
        required true
    }
}
"#,
        );
        write_wiki_page(dir.path(), "index.md", "ProGit");
        write_wiki_page(dir.path(), "other.md", "Other");
        write_issue(dir.path(), "issue.json", &Issue::new("Valid issue"));

        let report = validate_project(dir.path()).unwrap();

        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|error| error.message.contains("must be declared by a page path")));
    }
}
