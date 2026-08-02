use std::env;
use std::io::IsTerminal;
use std::process::Command;

pub fn ensure_terminal() -> anyhow::Result<()> {
    // Check if we are running in a terminal
    if std::io::stdout().is_terminal() {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // We are run from GUI (mouse double-click). Spawn a terminal.
        let current_exe = env::current_exe()?;
        let current_exe_str = current_exe.to_string_lossy();

        // List of common terminal emulators on Linux
        let terminals = [
            ("gnome-terminal", vec!["--", &current_exe_str]),
            ("konsole", vec!["-e", &current_exe_str]),
            ("xfce4-terminal", vec!["-e", &current_exe_str]),
            ("x-terminal-emulator", vec!["-e", &current_exe_str]),
            ("xterm", vec!["-e", &current_exe_str]),
            ("kitty", vec!["-e", &current_exe_str]),
            ("alacritty", vec!["-e", &current_exe_str]),
        ];

        for (term, args) in terminals.iter() {
            if Command::new(term).args(args).spawn().is_ok() {
                // Successfully spawned terminal, exit the parent GUI process
                std::process::exit(0);
            }
        }
    }

    Ok(())
}

pub fn hold_terminal() {
    println!("\nNhấn [Enter] để thoát...");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}
