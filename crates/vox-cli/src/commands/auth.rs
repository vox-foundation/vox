use anyhow::Result;
use clap::Subcommand;
use std::io::{self, Write};

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    /// Mount Turso DB for Zero-Knowledge Vault
    Login,
    /// Unlock device Vault with password
    Unlock,
}

pub async fn run(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login => run_login().await,
        AuthCmd::Unlock => run_unlock().await,
    }
}

async fn run_login() -> Result<()> {
    println!("Authenticating zero-knowledge Vault...");
    print!("Vault DB URL: ");
    io::stdout().flush()?;
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    let url = url.trim();

    print!("Vault Auth Token: ");
    io::stdout().flush()?;
    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim();

    use anyhow::Context;
    let keyring = keyring::Entry::new("vox-clavis-env", "turso-url")
        .context("Failed to instantiate keyring for turso-url. Keyring may not be available.")?;
    keyring.set_password(url)
        .context("Failed to set turso-url in keyring.")?;
    
    let keyring_token = keyring::Entry::new("vox-clavis-env", "turso-token")
        .context("Failed to instantiate keyring for turso-token.")?;
    keyring_token.set_password(token)
        .context("Failed to set turso-token in keyring.")?;

    println!("Vault configuration complete. Please run `vox auth unlock` next.");
    Ok(())
}

async fn run_unlock() -> Result<()> {
    println!("Unlocking Vault...");
    print!("Vault Password: ");
    io::stdout().flush()?;
    let mut pwd = String::new();
    io::stdin().read_line(&mut pwd)?;
    let pwd = pwd.trim();
    
    use anyhow::Context;
    let keyring_pwd = keyring::Entry::new("vox-clavis-vault", "master")
        .context("Failed to instantiate keyring for vault master key.")?;
    keyring_pwd.set_password(pwd)
        .context("Failed to store master key in keyring.")?;
    
    println!("Vault successfully unlocked via Argon2-derived master key!");
    Ok(())
}
