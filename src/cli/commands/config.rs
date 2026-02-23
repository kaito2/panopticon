use anyhow::Result;
use std::path::Path;

use crate::cli::ConfigAction;
use crate::config::PanopticonConfig;

/// Handle config subcommands.
pub async fn handle(action: ConfigAction, state_dir: &Path) -> Result<()> {
    match action {
        ConfigAction::Init => {
            let path = PanopticonConfig::config_path(state_dir);
            if path.exists() {
                println!("Config already exists at: {}", path.display());
                println!("Use `panopticon config show` to view it.");
                return Ok(());
            }

            let config = PanopticonConfig::default();
            config.save(state_dir)?;
            println!("Created default config at: {}", path.display());
            println!();
            print_config(&config);
        }

        ConfigAction::Show => {
            let config = PanopticonConfig::load(state_dir)?;
            print_config(&config);
        }
    }
    Ok(())
}

fn print_config(config: &PanopticonConfig) {
    println!("Configuration:");
    println!("  state_dir:               {}", config.state_dir);
    println!("  default_model:           {}", config.default_model);
    println!("  permission_mode:         {}", config.permission_mode);
    println!("  max_turns:               {}", config.max_turns);
    println!("  min_reputation_threshold: {:.2}", config.min_reputation_threshold);
    println!("  decomposition_strategy:  {}", config.decomposition_strategy);
    if config.allowed_tools.is_empty() {
        println!("  allowed_tools:           (all)");
    } else {
        println!("  allowed_tools:           {}", config.allowed_tools.join(", "));
    }
}
