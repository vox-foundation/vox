//! Minimal `vox shell` REPL — native `pwd` / `ls` / `cat` plus passthrough to the OS shell.

use std::io::{self, Write};

use tokio::process::Command;

/// Run the `vox shell` REPL.
pub async fn run_shell() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║          Vox Shell                               ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Interactive shell — `pwd`, `ls`, `cat`, or OS cmd ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("Type 'exit' or 'quit' to leave.\n");

    let stdin = io::stdin();

    loop {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        print!("vox {} > ", cwd.display());
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if line == "exit" || line == "quit" {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "pwd" => {
                println!("{}", cwd.display());
            }
            "ls" => match tokio::fs::read_dir(&cwd).await {
                Ok(mut rd) => {
                    while let Ok(Some(entry)) = rd.next_entry().await {
                        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                        let type_str = if is_dir { "DIR " } else { "FILE" };
                        println!("  {} {}", type_str, entry.file_name().to_string_lossy());
                    }
                }
                Err(e) => eprintln!("Error listing directory: {e}"),
            },
            "cat" => {
                if args.is_empty() {
                    eprintln!("Usage: cat <file>");
                } else {
                    let path = cwd.join(args[0]);
                    match tokio::fs::read_to_string(&path).await {
                        Ok(content) => print!("{content}"),
                        Err(e) => eprintln!("Error reading file: {e}"),
                    }
                }
            }
            _ => match Command::new(cmd).args(args).status().await {
                Ok(status) if !status.success() => {
                    eprintln!("Command exited with status: {status}");
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    eprintln!("vox shell: command not found: {cmd}");
                }
                Err(e) => eprintln!("{e}"),
                Ok(_) => {}
            },
        }
    }

    Ok(())
}
