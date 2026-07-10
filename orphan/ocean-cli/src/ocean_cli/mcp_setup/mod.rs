pub mod agents;
pub mod detect;
pub mod write;

use std::path::PathBuf;

use agents::AgentType;
use detect::detect_all;
use write::write_agent_config;

pub fn cmd_mcp_setup(agent: Option<String>, write_mode: bool) -> Result<(), String> {
    let binary = resolve_mcp_binary()?;

    let configs = if let Some(name) = agent {
        let agent_type = AgentType::from_str(&name)
            .ok_or_else(|| format!("Unknown agent '{}'. Supported agents: {}", name, agents::AgentType::known_names().join(", ")))?;
        vec![detect::detect_agent(agent_type)]
    } else {
        detect_all()
    };

    let detected: Vec<_> = configs.iter().filter(|c| c.installed).collect();

    if detected.is_empty() {
        eprintln!("No supported AI agents detected on this system.");
        eprintln!("Run `ocean mcp setup <agent>` to target a specific agent.");
        eprintln!("Supported: {}", agents::AgentType::known_names().join(", "));
        return Ok(());
    }

    if write_mode {
        for config in &detected {
            write_agent_config(config, &binary)?;
        }
    } else {
        for config in &detected {
            println!("{}", write::generate_config_block_display(config, &binary));
            println!();
        }

        eprintln!("---");
        eprintln!("Pass --write to automatically write these config files.");
    }

    Ok(())
}

fn resolve_mcp_binary() -> Result<String, String> {
    for name in &["ocean_mcp", "mcp", "ocean-mcp"] {
        if find_in_path(name) {
            return Ok(name.to_string());
        }
    }

    if find_in_path("cargo") {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()
            .or_else(|| {
                std::env::current_dir().ok().and_then(|cwd| find_cargo_toml(cwd))
            });
        if let Some(dir) = manifest_dir {
            eprintln!("Warning: 'ocean_mcp' binary not found in PATH.");
            eprintln!("  Run `cargo install --path \"{}\"` to install it, or use the full path.", dir);
            eprintln!("  Using 'ocean_mcp' as the command name for config generation (adjust if needed).");
            return Ok("ocean_mcp".to_string());
        }
    }

    Err(
        "MCP binary not found in PATH. Install it with `cargo install --path .` or ensure 'ocean_mcp' is in your PATH."
            .to_string(),
    )
}

fn find_in_path(name: &str) -> bool {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let exe = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    for dir in std::env::split_paths(&path_var) {
        if dir.join(&exe).exists() {
            return true;
        }
    }
    false
}

fn find_cargo_toml(dir: PathBuf) -> Option<String> {
    let mut current = Some(dir.as_path());
    while let Some(path) = current {
        if path.join("Cargo.toml").exists() {
            return path.to_str().map(|s| s.to_string());
        }
        current = path.parent();
    }
    None
}
