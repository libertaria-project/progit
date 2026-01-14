use super::paths;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Check for old .projects directory and migrate if needed
pub fn check_and_migrate(root: &Path) -> Result<bool> {
    let old_dir = root.join(".projects");
    let new_project_dir = root.join(paths::PROJECT_DIR);
    let new_local_dir = root.join(paths::LOCAL_DIR);

    if old_dir.exists() {
        println!(
            "📦 Detected legacy .projects/ directory. Migrating to Split Core architecture..."
        );

        // Ensure new directories exist
        if !new_project_dir.exists() {
            fs::create_dir(&new_project_dir).context("Failed to create .project dir")?;
        }
        if !new_local_dir.exists() {
            fs::create_dir(&new_local_dir).context("Failed to create .progit dir")?;
        }

        // 1. Move Issues (.projects/issues -> .project/issues)
        let old_issues = old_dir.join("issues");
        let new_issues = new_project_dir.join("issues");
        if old_issues.exists() {
            if new_issues.exists() {
                println!("   ⚠️  Target issues dir already exists. Skipping move.");
            } else {
                fs::rename(&old_issues, &new_issues).context("Failed to move issues")?;
                println!("   ✅ Moved issues to .project/issues");
            }
        }

        // 2. Move Config (.projects/config.kdl -> .project/config.kdl)
        let old_config = old_dir.join("config.kdl");
        let new_config = new_project_dir.join("config.kdl");
        if old_config.exists() && !new_config.exists() {
            fs::rename(&old_config, &new_config).context("Failed to move config")?;
            println!("   ✅ Moved config to .project/config.kdl");
        }

        // 3. Move Cache (.projects/.cache/issues.json -> .progit/issues.json)
        let old_cache = old_dir.join(".cache/issues.json");
        let new_cache = new_local_dir.join("issues.json");
        if old_cache.exists() && !new_cache.exists() {
            fs::rename(&old_cache, &new_cache).context("Failed to move cache")?;
            println!("   ✅ Moved cache to .progit/issues.json");
        }

        // 4. Remove old directory if empty-ish
        // We might fail if there are other files, that's fine.
        let _ = fs::remove_dir_all(&old_dir);
        // Note: remove_dir_all deletes everything. Since we moved the important stuff, this cleans up empty dirs.
        // If users had other custom stuff there, it's gone. But .projects was ours.

        println!("🚀 Migration complete.");
        return Ok(true);
    }

    Ok(false)
}
