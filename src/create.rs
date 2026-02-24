use std::path::Path;
use std::process::Command;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::github;

#[derive(Clone, Copy)]
pub enum Language {
    NodeJs,
    Python,
    Mixed,
}

impl Language {
    fn template_prefix(&self) -> &'static str {
        match self {
            Language::NodeJs => "nodejs",
            Language::Python => "python",
            Language::Mixed => "mixed",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Language::NodeJs => "TypeScript/Node.js (requires Node.js 18+)",
            Language::Python => "Python (requires Python 3.10+, uv)",
            Language::Mixed => "Mixed (Node.js + Python, requires both)",
        }
    }
}

fn is_valid_folder_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Project folder name cannot be empty.");
    }
    if name.contains("..") {
        return Err("Project folder name cannot contain '..'.");
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return Err("Project folder name cannot be an absolute path.");
    }
    if name.contains('/') || name.contains('\\') {
        return Err("Project folder name cannot contain path separators.");
    }
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
    for ch in invalid_chars {
        if name.contains(ch) {
            return Err("Project folder name contains invalid characters.");
        }
    }
    Ok(())
}

fn is_dir_empty(path: &Path) -> bool {
    path.read_dir()
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

fn check_prerequisite(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub async fn run(name_arg: Option<String>, force: bool) -> anyhow::Result<()> {
    crate::banner::print();

    let folder_name = match name_arg {
        Some(n) if !n.is_empty() => n,
        _ => {
            let name = dialoguer::Input::<String>::new()
                .with_prompt("  Project folder name")
                .default("my-motia-project".into())
                .interact()?;
            if name.is_empty() {
                anyhow::bail!("Project folder name is required.");
            }
            name
        }
    };

    if folder_name != "." {
        if let Err(msg) = is_valid_folder_name(&folder_name) {
            anyhow::bail!("{}", msg);
        }
    }

    let target_dir = std::env::current_dir()?.join(&folder_name);

    if target_dir.exists() {
        if is_dir_empty(&target_dir) {
            println!(
                "\n  {} Directory \"{}\" exists but is empty, using it.\n",
                "ℹ".blue(),
                folder_name
            );
        } else if force {
            println!(
                "\n  {} Using existing directory \"{}\" (--force).\n",
                "⚠".yellow(),
                folder_name
            );
        } else {
            let action_idx = dialoguer::Select::new()
                .with_prompt(format!(
                    "  Directory \"{}\" already exists. What would you like to do?",
                    folder_name
                ))
                .items(&[
                    "Use existing directory (merge template files into it)",
                    "Overwrite (delete existing contents first)",
                    "Cancel",
                ])
                .default(0)
                .interact()?;

            match action_idx {
                0 => {
                    println!(
                        "\n  {} Merging template into existing directory.\n",
                        "ℹ".blue(),
                    );
                }
                1 => {
                    println!(
                        "\n  {} Removing existing directory contents...\n",
                        "⚠".yellow(),
                    );
                    std::fs::remove_dir_all(&target_dir)?;
                }
                _ => {
                    println!("\n  Cancelled.\n");
                    return Ok(());
                }
            }
        }
    }

    let lang_idx = dialoguer::Select::new()
        .with_prompt("  Select language")
        .items(&[
            Language::NodeJs.display_name(),
            Language::Python.display_name(),
            Language::Mixed.display_name(),
        ])
        .default(0)
        .interact()?;

    let lang = match lang_idx {
        0 => Language::NodeJs,
        1 => Language::Python,
        _ => Language::Mixed,
    };

    check_prerequisites(lang)?;

    let has_iii = dialoguer::Confirm::new()
        .with_prompt("  Do you have iii installed?")
        .default(true)
        .interact()?;

    if !has_iii {
        println!();
        println!("  Motia is now powered by iii for step orchestration.");
        println!("  iii is the backend engine that runs your Motia steps,");
        println!("  handling APIs, queues, state, and workflows in a single runtime.");
        println!();
        println!(
            "  Install iii → {}https://iii.dev/docs{}",
            "\x1b[1m", "\x1b[0m"
        );
        println!();
    }

    let display_name = if folder_name == "." {
        "current directory".to_string()
    } else {
        format!("./{}", folder_name)
    };
    println!();
    println!("  Creating project in {}", display_name);
    println!();

    let client = reqwest::Client::builder()
        .user_agent("motia-cli")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let template_prefix = lang.template_prefix();
    let files = match github::fetch_tree(&client, template_prefix).await {
        Ok(f) if f.is_empty() => {
            anyhow::bail!(
                "No template files found for {}. Check your network connection and try again.",
                lang.display_name()
            );
        }
        Ok(f) => f,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("403") {
                anyhow::bail!(
                    "GitHub API rate limit exceeded. Wait a few minutes and try again, \
                     or set GITHUB_TOKEN to increase your rate limit."
                );
            }
            anyhow::bail!("Failed to fetch template: {}", e);
        }
    };

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.dim} [{bar:20.cyan/dim}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("━╸─"),
    );

    let mut dirs = std::collections::HashSet::new();
    for path in &files {
        let rel_path = strip_template_prefix(path, template_prefix);
        if let Some(parent) = Path::new(&rel_path).parent() {
            let parent_str = parent.to_string_lossy();
            if !parent_str.is_empty() {
                dirs.insert(parent_str.to_string());
            }
        }
    }

    std::fs::create_dir_all(&target_dir)?;
    for dir in &dirs {
        std::fs::create_dir_all(target_dir.join(dir))?;
    }

    let mut failed_downloads: Vec<String> = Vec::new();

    for (i, path) in files.iter().enumerate() {
        let rel_path = strip_template_prefix(path, template_prefix);
        pb.set_message(rel_path.clone());
        pb.set_position((i + 1) as u64);

        let content = match github::download_file(&client, path).await {
            Ok(c) => c,
            Err(e) => {
                failed_downloads.push(format!("{} ({})", rel_path, e));
                continue;
            }
        };

        let out_path = target_dir.join(&rel_path);
        let mut final_content = content;

        if rel_path.ends_with("package.json") {
            if let Ok(mut pkg) = serde_json::from_str::<serde_json::Value>(&final_content) {
                let name = if rel_path.contains("nodejs/") {
                    format!("{}-nodejs", folder_name)
                } else if folder_name == "." {
                    std::env::current_dir()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "my-motia-project".to_string())
                } else {
                    folder_name.clone()
                };
                pkg["name"] = serde_json::Value::String(name);
                final_content = serde_json::to_string_pretty(&pkg)?;
            }
        }

        std::fs::write(&out_path, final_content)?;
    }

    pb.finish_and_clear();

    if !failed_downloads.is_empty() {
        println!(
            "  {} Some files failed to download:",
            "⚠".yellow()
        );
        for f in &failed_downloads {
            println!("    - {}", f);
        }
        println!();
    }

    println!("  Installing dependencies...");
    println!();

    match lang {
        Language::NodeJs => {
            run_install(&target_dir, "npm", &["install"])?;
        }
        Language::Python => {
            run_install(&target_dir, "uv", &["sync"])?;
        }
        Language::Mixed => {
            run_install(&target_dir.join("nodejs"), "npm", &["install"])?;
            run_install(&target_dir.join("python"), "uv", &["sync"])?;
        }
    }

    println!();
    println!("  {} Project created successfully!", "✓".green());
    println!();
    println!("  Next steps:");
    if folder_name != "." {
        println!("    cd {}", folder_name);
    }
    println!("    iii -c iii-config.yaml");
    println!();

    Ok(())
}

fn strip_template_prefix(path: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        path.to_string()
    } else {
        path.strip_prefix(&format!("{}/", prefix))
            .unwrap_or(path)
            .to_string()
    }
}

fn check_prerequisites(lang: Language) -> anyhow::Result<()> {
    let mut missing: Vec<&str> = Vec::new();

    match lang {
        Language::NodeJs => {
            if !check_prerequisite("node") {
                missing.push("node (v18+)");
            }
            if !check_prerequisite("npm") {
                missing.push("npm");
            }
        }
        Language::Python => {
            if !check_prerequisite("python3") {
                missing.push("python3 (v3.10+)");
            }
            if !check_prerequisite("uv") {
                missing.push("uv");
            }
        }
        Language::Mixed => {
            if !check_prerequisite("node") {
                missing.push("node (v18+)");
            }
            if !check_prerequisite("npm") {
                missing.push("npm");
            }
            if !check_prerequisite("python3") {
                missing.push("python3 (v3.10+)");
            }
            if !check_prerequisite("uv") {
                missing.push("uv");
            }
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Missing required tools: {}. Please install them before continuing.",
            missing.join(", ")
        );
    }

    Ok(())
}

fn run_install(cwd: &Path, cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run '{}': {}. Is it installed?", cmd, e))?;

    if !status.success() {
        anyhow::bail!(
            "'{}' failed with exit code {}.",
            cmd,
            status.code().map_or("unknown".to_string(), |c| c.to_string())
        );
    }
    Ok(())
}
