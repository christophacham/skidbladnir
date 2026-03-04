//! Per-transition side effect logic for workflow phase changes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use agtx_core::config::WorkflowPlugin;
use agtx_core::db::{Database, Task};
use agtx_core::git::{
    GitOperations, GitProviderOperations, PullRequestState, RealGitHubOps, RealGitOps,
};
use agtx_core::skills;

use crate::session::{SessionManager, SpawnRequest};

/// Generate a task slug suitable for branch names and directory names.
fn generate_task_slug(id: &str, title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-');
    let slug: String = slug.chars().take(30).collect();
    let short_id = if id.len() >= 8 { &id[..8] } else { id };
    format!("{}-{}", short_id, slug)
}

/// Resolve the plugin for a task, with fallback to bundled.
fn resolve_plugin_sync(task: &Task, project_path: Option<&Path>) -> Option<WorkflowPlugin> {
    let plugin_name = task.plugin.as_deref().unwrap_or("agtx");
    WorkflowPlugin::load(plugin_name, project_path)
        .ok()
        .or_else(|| skills::load_bundled_plugin(plugin_name))
}

/// Resolve the skill command for a given phase and agent.
fn resolve_skill_command(plugin: &WorkflowPlugin, phase: &str, agent_name: &str) -> Option<String> {
    let canonical = match phase {
        "planning" => plugin.commands.planning.as_deref(),
        "running" => plugin.commands.running.as_deref(),
        "review" => plugin.commands.review.as_deref(),
        "research" => plugin.commands.research.as_deref(),
        _ => None,
    }?;
    skills::transform_plugin_command(canonical, agent_name)
}

/// Resolve the prompt for a given phase, substituting placeholders.
fn resolve_prompt(
    plugin: &WorkflowPlugin,
    phase: &str,
    task_content: &str,
    task_id: &str,
    _cycle: i32,
) -> Option<String> {
    let template = match phase {
        "planning" => plugin.prompts.planning.as_deref(),
        "running" => plugin.prompts.running.as_deref(),
        "review" => plugin.prompts.review.as_deref(),
        "research" => plugin.prompts.research.as_deref(),
        _ => None,
    }?;
    Some(
        template
            .replace("{task}", task_content)
            .replace("{task_id}", task_id)
            .replace("{phase}", phase),
    )
}

/// Deploy skills to agent-native paths inside the worktree.
fn write_skills_to_worktree(worktree_path: &Path, plugin_name: &str, _project_path: Option<&Path>) {
    // 1. Write canonical skills to .agtx/skills/
    let canonical_base = worktree_path.join(".agtx").join("skills");
    for (dir_name, content) in skills::DEFAULT_SKILLS {
        let skill_dir = canonical_base.join(dir_name);
        let _ = std::fs::create_dir_all(&skill_dir);
        let _ = std::fs::write(skill_dir.join("SKILL.md"), content);
    }

    // 2. Write to agent-native paths for each agent type
    let agents = ["claude", "gemini", "opencode", "codex", "copilot"];
    for agent in &agents {
        let Some((base_dir, namespace)) = skills::agent_native_skill_dir(agent) else {
            continue;
        };

        for (dir_name, content) in skills::DEFAULT_SKILLS {
            match *agent {
                "codex" => {
                    // .codex/skills/{dir_name}/SKILL.md
                    let skill_dir = worktree_path.join(base_dir).join(dir_name);
                    let _ = std::fs::create_dir_all(&skill_dir);
                    let _ = std::fs::write(skill_dir.join("SKILL.md"), content);
                }
                "gemini" => {
                    // .gemini/commands/{namespace}/{name}.toml
                    let dir = worktree_path.join(base_dir).join(namespace);
                    let _ = std::fs::create_dir_all(&dir);
                    let filename = skills::skill_dir_to_filename(dir_name, agent);
                    let description = skills::extract_description(content)
                        .unwrap_or_else(|| dir_name.replace('-', " "));
                    let toml_content = skills::skill_to_gemini_toml(&description, content);
                    let _ = std::fs::write(dir.join(filename), toml_content);
                }
                "opencode" => {
                    // .config/opencode/command/{dir_name}.md (no frontmatter)
                    // opencode base_dir returns ".opencode/commands", but actual path is .config/opencode/command
                    let dir = worktree_path
                        .join(".config")
                        .join("opencode")
                        .join("command");
                    let _ = std::fs::create_dir_all(&dir);
                    let filename = skills::skill_dir_to_filename(dir_name, agent);
                    let stripped = skills::strip_frontmatter(content);
                    let _ = std::fs::write(dir.join(filename), stripped);
                }
                _ => {
                    // claude, copilot: .{agent}/commands/{namespace}/{name}.md
                    let dir = worktree_path.join(base_dir).join(namespace);
                    let _ = std::fs::create_dir_all(&dir);
                    let filename = skills::skill_dir_to_filename(dir_name, agent);
                    let _ = std::fs::write(dir.join(filename), content);
                }
            }
        }
    }

    // 3. Write plugin-specific skills if they exist on disk
    if let Some(plugin_dir) = WorkflowPlugin::plugin_dir(plugin_name, _project_path) {
        let skills_dir = plugin_dir.join("skills");
        if skills_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let skill_file = path.join("SKILL.md");
                    if !skill_file.exists() {
                        continue;
                    }
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let content = match std::fs::read_to_string(&skill_file) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    // Canonical copy
                    let canon_dir = canonical_base.join(&dir_name);
                    let _ = std::fs::create_dir_all(&canon_dir);
                    let _ = std::fs::write(canon_dir.join("SKILL.md"), &content);

                    // Agent-native copies
                    for agent in &agents {
                        let Some((base_dir, namespace)) = skills::agent_native_skill_dir(agent)
                        else {
                            continue;
                        };
                        match *agent {
                            "codex" => {
                                let sd = worktree_path.join(base_dir).join(&dir_name);
                                let _ = std::fs::create_dir_all(&sd);
                                let _ = std::fs::write(sd.join("SKILL.md"), &content);
                            }
                            "gemini" => {
                                let dir = worktree_path.join(base_dir).join(namespace);
                                let _ = std::fs::create_dir_all(&dir);
                                let filename = skills::skill_dir_to_filename(&dir_name, agent);
                                let desc = skills::extract_description(&content)
                                    .unwrap_or_else(|| dir_name.replace('-', " "));
                                let toml = skills::skill_to_gemini_toml(&desc, &content);
                                let _ = std::fs::write(dir.join(filename), toml);
                            }
                            "opencode" => {
                                let dir = worktree_path
                                    .join(".config")
                                    .join("opencode")
                                    .join("command");
                                let _ = std::fs::create_dir_all(&dir);
                                let filename = skills::skill_dir_to_filename(&dir_name, agent);
                                let stripped = skills::strip_frontmatter(&content);
                                let _ = std::fs::write(dir.join(filename), stripped);
                            }
                            _ => {
                                let dir = worktree_path.join(base_dir).join(namespace);
                                let _ = std::fs::create_dir_all(&dir);
                                let filename = skills::skill_dir_to_filename(&dir_name, agent);
                                let _ = std::fs::write(dir.join(filename), &content);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Transition: Backlog -> Planning
///
/// Creates git worktree, deploys skills, spawns PTY session, sends commands.
/// Runs as a background task -- the caller updates status immediately.
pub async fn backlog_to_planning(
    task: Task,
    project_path: Option<PathBuf>,
    session_manager: Arc<SessionManager>,
    db_path: PathBuf,
) -> Result<()> {
    let slug = generate_task_slug(&task.id, &task.title);
    let pp = project_path.clone();
    let plugin_name = task.plugin.clone().unwrap_or_else(|| "agtx".to_string());

    // Resolve plugin
    let plugin = {
        let task_clone = task.clone();
        let pp2 = pp.clone();
        tokio::task::spawn_blocking(move || resolve_plugin_sync(&task_clone, pp2.as_deref()))
            .await
            .context("spawn_blocking failed")?
    };

    // Create worktree
    let worktree_path = if let Some(ref pp) = pp {
        let project = pp.clone();
        let s = slug.clone();
        tokio::task::spawn_blocking(move || -> Result<String> {
            RealGitOps.create_worktree(&project, &s)
        })
        .await
        .context("spawn_blocking failed")??
    } else {
        bail!("Cannot create worktree without project path");
    };

    let wt_path = PathBuf::from(&worktree_path);

    // Initialize worktree (copy files, run init script)
    if let Some(ref pp) = pp {
        let project = pp.clone();
        let wt = wt_path.clone();
        let copy_files = plugin.as_ref().map(|p| p.copy_files.join(","));
        let init_script = plugin.as_ref().and_then(|p| p.init_script.clone());
        let copy_dirs = plugin
            .as_ref()
            .map(|p| p.copy_dirs.clone())
            .unwrap_or_default();

        tokio::task::spawn_blocking(move || {
            RealGitOps.initialize_worktree(&project, &wt, copy_files, init_script, copy_dirs);
        })
        .await
        .context("spawn_blocking failed")?;
    }

    // Deploy skills
    {
        let wt = wt_path.clone();
        let pn = plugin_name.clone();
        let pp3 = pp.clone();
        tokio::task::spawn_blocking(move || {
            write_skills_to_worktree(&wt, &pn, pp3.as_deref());
        })
        .await
        .context("spawn_blocking failed")?;
    }

    // Build agent command
    let agent = agtx_core::agent::get_agent(&task.agent)
        .unwrap_or_else(|| agtx_core::agent::known_agents().into_iter().next().unwrap());
    let cmd_str = agent.build_interactive_command("");
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    let command = parts.first().unwrap_or(&"sh").to_string();
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Spawn PTY session
    let session_id = session_manager
        .spawn(SpawnRequest {
            command,
            args,
            working_dir: wt_path.clone(),
            env: vec![],
            cols: 120,
            rows: 40,
        })
        .await
        .context("Failed to spawn PTY session")?;

    // Update task with session_id, worktree_path, branch_name
    // We need to re-load the task since we're running in background
    let db_path2 = db_path.clone();
    let session_id_str = session_id.to_string();
    let wt_str = worktree_path.clone();
    let branch = format!("task/{}", slug);
    let tid = task.id.clone();
    let pn = plugin_name.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let db = Database::open_at(&db_path2)?;
        if let Some(mut t) = db.get_task(&tid)? {
            t.session_id = Some(session_id_str);
            t.worktree_path = Some(wt_str);
            t.branch_name = Some(branch);
            t.plugin = Some(pn);
            t.updated_at = Utc::now();
            db.update_task(&t)?;
        }
        Ok(())
    })
    .await
    .context("spawn_blocking failed")??;

    // Wait briefly for agent readiness before sending commands
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Send skill command + prompt
    if let Some(ref plugin) = plugin {
        let task_content = task.description.as_deref().unwrap_or(&task.title);
        if let Some(cmd) = resolve_skill_command(plugin, "planning", &agent.name) {
            let _ = session_manager.write(session_id, &cmd).await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        if let Some(prompt) = resolve_prompt(plugin, "planning", task_content, &task.id, task.cycle)
        {
            let _ = session_manager.write(session_id, &prompt).await;
        }
    }

    Ok(())
}

/// Transition: Planning -> Running
///
/// Sends the execute command/prompt to the existing session.
pub async fn planning_to_running(
    task: &mut Task,
    project_path: Option<&Path>,
    session_manager: &SessionManager,
    _db_path: &Path,
) -> Result<()> {
    let plugin = {
        let t = task.clone();
        let pp = project_path.map(|p| p.to_path_buf());
        tokio::task::spawn_blocking(move || resolve_plugin_sync(&t, pp.as_deref()))
            .await
            .context("spawn_blocking failed")?
    };

    if let Some(ref plugin) = plugin {
        let agent = agtx_core::agent::get_agent(&task.agent)
            .unwrap_or_else(|| agtx_core::agent::known_agents().into_iter().next().unwrap());

        if let Some(sid) = task.session_id.as_ref() {
            let uuid = uuid::Uuid::parse_str(sid).context("Invalid session UUID")?;
            let task_content = task.description.as_deref().unwrap_or(&task.title);

            if let Some(cmd) = resolve_skill_command(plugin, "running", &agent.name) {
                let _ = session_manager.write(uuid, &cmd).await;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            if let Some(prompt) =
                resolve_prompt(plugin, "running", task_content, &task.id, task.cycle)
            {
                let _ = session_manager.write(uuid, &prompt).await;
            }
        }
    }

    Ok(())
}

/// Transition: Running -> Review
///
/// Updates status. Copies back files if configured.
pub async fn running_to_review(task: &mut Task, project_path: Option<&Path>) -> Result<()> {
    // Copy-back files if plugin has review entries
    let plugin = {
        let t = task.clone();
        let pp = project_path.map(|p| p.to_path_buf());
        tokio::task::spawn_blocking(move || resolve_plugin_sync(&t, pp.as_deref()))
            .await
            .context("spawn_blocking failed")?
    };

    if let Some(ref plugin) = plugin {
        if let Some(files) = plugin.copy_back.get("review") {
            if let (Some(wt), Some(pp)) = (task.worktree_path.as_ref(), project_path) {
                let wt_path = PathBuf::from(wt);
                let project = pp.to_path_buf();
                let files = files.clone();
                tokio::task::spawn_blocking(move || {
                    for file in &files {
                        let src = wt_path.join(file);
                        let dst = project.join(file);
                        if src.is_dir() {
                            let _ = copy_dir_recursive(&src, &dst);
                        } else if src.exists() {
                            if let Some(parent) = dst.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::copy(&src, &dst);
                        }
                    }
                })
                .await
                .context("spawn_blocking failed")?;
            }
        }
    }

    Ok(())
}

/// Transition: Review -> Done
///
/// Checks PR status, kills session, removes worktree.
/// Returns optional warning string (e.g., "PR is still open").
pub async fn review_to_done(
    task: &mut Task,
    project_path: Option<&Path>,
    session_manager: &SessionManager,
) -> Result<Option<String>> {
    let mut warning = None;

    // Check PR status if set
    if let (Some(pr_num), Some(pp)) = (task.pr_number, project_path) {
        let project: PathBuf = pp.to_path_buf();
        let state: PullRequestState =
            tokio::task::spawn_blocking(move || RealGitHubOps.get_pr_state(&project, pr_num))
                .await
                .context("spawn_blocking failed")??;

        if state == PullRequestState::Open {
            warning = Some("PR is still open".to_string());
        }
    }

    // Kill PTY session if still alive
    if let Some(sid) = task.session_id.as_ref() {
        if let Ok(uuid) = uuid::Uuid::parse_str(sid) {
            let _ = session_manager.kill(uuid).await;
        }
    }
    task.session_id = None;

    // Remove worktree (keep branch)
    if let (Some(wt), Some(pp)) = (task.worktree_path.as_ref(), project_path) {
        let project: PathBuf = pp.to_path_buf();
        let wt_path: String = wt.clone();
        let _ = tokio::task::spawn_blocking(move || RealGitOps.remove_worktree(&project, &wt_path))
            .await;
    }
    task.worktree_path = None;

    Ok(warning)
}

/// Transition: Review -> Planning (cyclic)
///
/// Increments cycle counter and sends planning command for new cycle.
pub async fn review_to_planning_cyclic(
    task: &mut Task,
    project_path: Option<&Path>,
    session_manager: &SessionManager,
    _db_path: &Path,
) -> Result<()> {
    let plugin = {
        let t = task.clone();
        let pp = project_path.map(|p| p.to_path_buf());
        tokio::task::spawn_blocking(move || resolve_plugin_sync(&t, pp.as_deref()))
            .await
            .context("spawn_blocking failed")?
    };

    if let Some(ref plugin) = plugin {
        let agent = agtx_core::agent::get_agent(&task.agent)
            .unwrap_or_else(|| agtx_core::agent::known_agents().into_iter().next().unwrap());

        if let Some(sid) = task.session_id.as_ref() {
            let uuid = uuid::Uuid::parse_str(sid).context("Invalid session UUID")?;
            let task_content = task.description.as_deref().unwrap_or(&task.title);

            if let Some(cmd) = resolve_skill_command(plugin, "planning", &agent.name) {
                let _ = session_manager.write(uuid, &cmd).await;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            if let Some(prompt) =
                resolve_prompt(plugin, "planning", task_content, &task.id, task.cycle)
            {
                let _ = session_manager.write(uuid, &prompt).await;
            }
        }
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
