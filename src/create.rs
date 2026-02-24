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

pub async fn run(name_arg: Option<String>) -> anyhow::Result<()> {
    crate::banner::print();

    let folder_name = match name_arg {
        Some(n) if !n.is_empty() => n,
        _ => {
            let name = dialoguer::Input::<String>::new()
                .with_prompt("  Project folder name")
                .default("my-motia-project".into())
                .interact()?;
            if name.is_empty() {
                eprintln!("\n  Project folder name is required.\n");
                std::process::exit(1);
            }
            name
        }
    };

    let target_dir = std::env::current_dir()?.join(&folder_name);
    if target_dir.exists() {
        eprintln!("\n  Directory \"{}\" already exists.\n", folder_name);
        std::process::exit(1);
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
        println!("  Install iii → {}https://iii.dev/docs{}", "\x1b[1m", "\x1b[0m");
        println!();
    }

    println!();
    println!("  Creating project in ./{}", folder_name);
    println!();

    let client = reqwest::Client::new();
    let template_prefix = lang.template_prefix();
    let files = github::fetch_tree(&client, template_prefix).await?;

    if files.is_empty() {
        anyhow::bail!("No template files found");
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.dim} {msg}")
            .unwrap()
            .progress_chars("▸▹▹▹"),
    );

    let mut dirs = std::collections::HashSet::new();
    for path in &files {
        let rel_path = if template_prefix.is_empty() {
            path.clone()
        } else {
            path.strip_prefix(&format!("{}/", template_prefix))
                .unwrap_or(path)
                .to_string()
        };
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

    for (i, path) in files.iter().enumerate() {
        pb.set_message(path.clone());
        pb.set_position((i + 1) as u64);

        let content = github::download_file(&client, path).await?;
        let rel_path = if template_prefix.is_empty() {
            path.clone()
        } else {
            path.strip_prefix(&format!("{}/", template_prefix))
                .unwrap_or(path)
                .to_string()
        };

        let out_path = target_dir.join(&rel_path);
        let mut final_content = content;

        if rel_path.ends_with("package.json") {
            if let Ok(mut pkg) = serde_json::from_str::<serde_json::Value>(&final_content) {
                let name = if rel_path.contains("nodejs/") {
                    format!("{}-nodejs", folder_name)
                } else {
                    folder_name.clone()
                };
                pkg["name"] = serde_json::Value::String(name);
                final_content = serde_json::to_string_pretty(&pkg)?;
            }
        }

        std::fs::write(&out_path, final_content)?;
    }

    pb.finish_with_message("");

    println!();
    println!("  Installing dependencies...");
    println!();

    match lang {
        Language::NodeJs => {
            run_install(&target_dir, "npm install")?;
        }
        Language::Python => {
            run_install(&target_dir, "uv sync")?;
        }
        Language::Mixed => {
            run_install(&target_dir.join("nodejs"), "npm install")?;
            run_install(&target_dir.join("python"), "uv sync")?;
        }
    }

    println!();
    println!("  {} Project created successfully!", "✓".green());
    println!();
    println!("  Next steps:");
    println!("    cd {}", folder_name);
    println!("    iii -c iii-config.yaml");
    println!();

    Ok(())
}

fn run_install(cwd: &Path, cmd: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let (cmd, args) = parts.split_first().unwrap_or((&"", &[]));
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()?;
    if !status.success() {
        anyhow::bail!("{} failed with exit code {:?}", cmd, status.code());
    }
    Ok(())
}
