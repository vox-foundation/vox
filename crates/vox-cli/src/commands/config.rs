use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Get a value from the layered config.
    Get {
        /// The config key to get.
        key: String,
    },
    /// Set a value in the local `~/.vox/config.toml` file.
    Set {
        /// The config key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Unset a value in the local `~/.vox/config.toml` file.
    Unset {
        /// The config key to unset.
        key: String,
    },
    /// List all the configuration entries explicitly set in `config.toml`.
    List,
    /// Synchronize settings with the cross-device account_config table.
    Sync {
        /// Push local `config.toml` settings to the sovereign account database.
        #[arg(long, conflicts_with = "pull")]
        push: bool,
        /// Pull account settings from the database down to `config.toml`.
        #[arg(long, conflicts_with = "push")]
        pull: bool,
    },
}

pub async fn run(cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Get { key } => {
            // Check env and TOML
            let val = vox_config::env_parse::resolve_config_str(&key, "<not set>");
            println!("{}", val);
        }
        ConfigCmd::Set { key, value } => {
            vox_config::toml_config::set_user_config_value(&key, &value).map_err(|e| anyhow::anyhow!(e))?;
            crate::diagnostics::print_success(&format!("Set {} = {}", key, value));
        }
        ConfigCmd::Unset { key } => {
            let removed = vox_config::toml_config::unset_user_config_value(&key).map_err(|e| anyhow::anyhow!(e))?;
            if removed {
                crate::diagnostics::print_success(&format!("Unset {}", key));
            } else {
                println!("Key {} was not set locally.", key);
            }
        }
        ConfigCmd::List => {
            let conf = vox_config::toml_config::load_user_config();
            if conf.values.is_empty() {
                println!("No explicit configuration set in ~/.vox/config.toml");
                return Ok(());
            }
            let mut keys: Vec<_> = conf.values.keys().collect();
            keys.sort();
            for k in keys {
                if let Some(v) = conf.values.get(k) {
                    if let Some(s) = v.as_str() {
                        println!("{} = {}", k, s);
                    } else {
                        println!("{} = {}", k, v);
                    }
                }
            }
        }
        ConfigCmd::Sync { push, pull } => {
            run_sync(push, pull).await?;
        }
    }
    Ok(())
}

async fn run_sync(push: bool, pull: bool) -> Result<()> {
    use vox_db::VoxDb;
    use anyhow::Context;

    // Use current computer's local user context or specific account 
    // Usually tied to Clavis Vault user identifier...
    let account_id = "local_account_sync"; 
    let db = VoxDb::connect_default().await?;

    if push {
        println!("Pushing local ~/.vox/config.toml settings to the account database...");
        let conf = vox_config::toml_config::load_user_config();
        for (k, v) in conf.values.iter() {
            if let Some(v_str) = v.as_str() {
                db.set_account_config(account_id, k, v_str)
                    .await
                    .context(format!("Failed to sync key {} to remote", k))?;
            } else {
                db.set_account_config(account_id, k, &v.to_string())
                    .await
                    .context(format!("Failed to sync key {} to remote", k))?;
            }
        }
        crate::diagnostics::print_success("Sync push completed successfully.");
    } else if pull {
        println!("Pulling settings from the remote account database...");
        let rows = db.list_account_configs(account_id, None).await?;
        let mut count = 0;
        for (k, v) in rows {
            vox_config::toml_config::set_user_config_value(&k, &v).map_err(|e| anyhow::anyhow!(e))?;
            count += 1;
        }
        crate::diagnostics::print_success(&format!("Sync pull completed. {} keys updated.", count));
    } else {
        println!("Please specify either --push or --pull for sync operations.\nExample: vox config sync --pull");
    }

    Ok(())
}
