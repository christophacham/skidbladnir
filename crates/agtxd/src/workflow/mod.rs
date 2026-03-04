pub mod artifacts;
pub mod transitions;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use agtx_core::config::WorkflowPlugin;
use agtx_core::db::{Database, Task, TaskStatus};
use agtx_core::skills;

use crate::session::SessionManager;

/// Response from advance_task including optional warnings
pub struct AdvanceResult {
    pub task: Task,
    pub warning: Option<String>,
}

/// Orchestrates all phase transitions with their side effects.
pub struct WorkflowService {
    pub session_manager: Arc<SessionManager>,
    pub db_path: PathBuf,
    pub global_db_path: PathBuf,
}

impl WorkflowService {
    pub fn new(
        session_manager: Arc<SessionManager>,
        db_path: PathBuf,
        global_db_path: PathBuf,
    ) -> Self {
        Self {
            session_manager,
            db_path,
            global_db_path,
        }
    }

    /// Advance a task to the next state.
    ///
    /// `direction` is "next" for normal progression or "cycle" for Review -> Planning.
    pub async fn advance_task(&self, task_id: &str, direction: &str) -> Result<AdvanceResult> {
        // Load task from DB
        let db_path = self.db_path.clone();
        let global_db_path = self.global_db_path.clone();
        let tid = task_id.to_string();

        let (task, project_path) = tokio::task::spawn_blocking(move || -> Result<_> {
            let db = Database::open_at(&db_path)?;
            let task = db
                .get_task(&tid)
                .context("DB error")?
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", tid))?;

            // Load project from global DB to get project_path
            let global_db = Database::open_global_at(&global_db_path)?;
            let projects = global_db.get_all_projects()?;
            let project = projects.iter().find(|p| p.id == task.project_id).cloned();
            let project_path = project.map(|p| PathBuf::from(&p.path));

            Ok((task, project_path))
        })
        .await
        .context("spawn_blocking failed")??;

        // Determine next status
        let (next_status, is_cycle) = match (task.status, direction) {
            (TaskStatus::Backlog, "next") => (TaskStatus::Planning, false),
            (TaskStatus::Planning, "next") => (TaskStatus::Running, false),
            (TaskStatus::Running, "next") => (TaskStatus::Review, false),
            (TaskStatus::Review, "next") => (TaskStatus::Done, false),
            (TaskStatus::Review, "cycle") => (TaskStatus::Planning, true),
            (TaskStatus::Done, _) => bail!("Cannot advance from Done"),
            (_, "cycle") => bail!("Cycle is only valid from Review"),
            _ => bail!("Invalid direction: {}", direction),
        };

        // If cyclic, verify plugin supports it
        if is_cycle {
            let plugin = self.resolve_plugin(&task, project_path.as_deref()).await?;
            if !plugin.map(|p| p.cyclic).unwrap_or(false) {
                bail!("Cycle not supported: plugin is not cyclic");
            }
        }

        // Execute transition side effects
        let mut updated_task = task.clone();
        updated_task.status = next_status;
        updated_task.updated_at = Utc::now();

        let mut warning = None;

        match (task.status, next_status) {
            (TaskStatus::Backlog, TaskStatus::Planning) => {
                // Heavy setup runs in background -- we update status immediately
                // and let the background task fill in session_id, worktree_path, etc.
                let bg_task = updated_task.clone();
                let bg_db_path = self.db_path.clone();
                let bg_sm = self.session_manager.clone();
                let bg_project_path = project_path.clone();

                tokio::spawn(async move {
                    if let Err(e) = transitions::backlog_to_planning(
                        bg_task,
                        bg_project_path,
                        bg_sm,
                        bg_db_path,
                    )
                    .await
                    {
                        tracing::error!("backlog_to_planning failed: {:#}", e);
                    }
                });
            }
            (TaskStatus::Planning, TaskStatus::Running) => {
                transitions::planning_to_running(
                    &mut updated_task,
                    project_path.as_deref(),
                    &self.session_manager,
                    &self.db_path,
                )
                .await?;
            }
            (TaskStatus::Running, TaskStatus::Review) => {
                transitions::running_to_review(&mut updated_task, project_path.as_deref()).await?;
            }
            (TaskStatus::Review, TaskStatus::Done) => {
                warning = transitions::review_to_done(
                    &mut updated_task,
                    project_path.as_deref(),
                    &self.session_manager,
                )
                .await?;
            }
            (TaskStatus::Review, TaskStatus::Planning) if is_cycle => {
                updated_task.cycle += 1;
                transitions::review_to_planning_cyclic(
                    &mut updated_task,
                    project_path.as_deref(),
                    &self.session_manager,
                    &self.db_path,
                )
                .await?;
            }
            _ => {}
        }

        // Save updated task to DB
        let db_path = self.db_path.clone();
        let save_task = updated_task.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Database::open_at(&db_path)?;
            db.update_task(&save_task)?;
            Ok(())
        })
        .await
        .context("spawn_blocking failed")??;

        Ok(AdvanceResult {
            task: updated_task,
            warning,
        })
    }

    /// Resolve the plugin for a task
    async fn resolve_plugin(
        &self,
        task: &Task,
        project_path: Option<&std::path::Path>,
    ) -> Result<Option<WorkflowPlugin>> {
        let plugin_name = task.plugin.clone().unwrap_or_else(|| "agtx".to_string());
        let pp = project_path.map(|p| p.to_path_buf());

        tokio::task::spawn_blocking(move || -> Result<Option<WorkflowPlugin>> {
            // Try loading from filesystem first (project-local, then global)
            if let Ok(plugin) = WorkflowPlugin::load(&plugin_name, pp.as_deref()) {
                return Ok(Some(plugin));
            }
            // Fall back to bundled
            Ok(skills::load_bundled_plugin(&plugin_name))
        })
        .await
        .context("spawn_blocking failed")?
    }
}
