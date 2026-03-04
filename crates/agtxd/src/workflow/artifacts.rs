//! Artifact polling task for detecting phase completion files.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agtx_core::config::WorkflowPlugin;
use agtx_core::db::{Database, TaskStatus};
use agtx_core::skills;

/// Check if a phase artifact file exists in the worktree.
fn phase_artifact_exists(
    worktree_path: &Path,
    phase: &str,
    plugin: &Option<WorkflowPlugin>,
) -> bool {
    let artifact_pattern = plugin.as_ref().and_then(|p| match phase {
        "planning" => p.artifacts.planning.as_deref(),
        "running" => p.artifacts.running.as_deref(),
        "review" => p.artifacts.review.as_deref(),
        "research" => p.artifacts.research.as_deref(),
        _ => None,
    });

    let Some(pattern) = artifact_pattern else {
        return false;
    };

    // Handle wildcards in pattern
    if pattern.contains('*') {
        // Simple glob: check if any file matches the pattern
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            let dir = worktree_path.join(Path::new(prefix).parent().unwrap_or(Path::new("")));
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let full = entry.path().to_string_lossy().to_string();
                    if full.ends_with(suffix) || name.ends_with(suffix.trim_start_matches('/')) {
                        return true;
                    }
                }
            }
        }
        false
    } else {
        worktree_path.join(pattern).exists()
    }
}

/// Background task that polls for artifact files.
///
/// Checks all tasks in Planning/Running status for artifact completion files.
/// Designed to be spawned as a tokio task.
pub async fn artifact_polling_task(db_path: PathBuf, interval: Duration) {
    let mut detected: HashSet<String> = HashSet::new();

    loop {
        tokio::time::sleep(interval).await;

        let db_path2 = db_path.clone();
        let tasks = match tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let db = Database::open_at(&db_path2)?;
            let mut active = db.get_tasks_by_status(TaskStatus::Planning)?;
            active.extend(db.get_tasks_by_status(TaskStatus::Running)?);
            Ok(active)
        })
        .await
        {
            Ok(Ok(tasks)) => tasks,
            _ => continue,
        };

        for task in &tasks {
            let Some(ref wt) = task.worktree_path else {
                continue;
            };

            // Skip if already detected
            let key = format!("{}:{}", task.id, task.status.as_str());
            if detected.contains(&key) {
                continue;
            }

            let wt_path = PathBuf::from(wt);
            let phase = task.status.as_str();
            let plugin_name = task.plugin.clone().unwrap_or_else(|| "agtx".to_string());

            let wt2 = wt_path.clone();
            let phase2 = phase.to_string();
            let pn2 = plugin_name.clone();

            let found = match tokio::task::spawn_blocking(move || {
                let plugin = WorkflowPlugin::load(&pn2, None)
                    .ok()
                    .or_else(|| skills::load_bundled_plugin(&pn2));
                phase_artifact_exists(&wt2, &phase2, &plugin)
            })
            .await
            {
                Ok(found) => found,
                Err(_) => continue,
            };

            if found {
                detected.insert(key);
                tracing::info!(
                    task_id = %task.id,
                    phase = %phase,
                    "Artifact detected for task"
                );
            }
        }
    }
}
