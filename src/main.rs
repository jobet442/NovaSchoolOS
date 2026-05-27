fn main() -> Result<(), std::io::Error> {
    // 1. Boot virtual hardware drivers
    drivers::init_drivers();

    // 2. Boot hybrid kernel components
    kernel::init_kernel();

    // Initialize networking stack
    networking::init_networking();

    // 3. Mount Virtual Filesystem partitions (NovaFS root, FAT32, EXT2 partitions)
    filesystem::init_vfs();

    // 4. Initialize userspace databases and classroom structures
    userspace::init_userspace();

    // 5. Initialize shell environment variables
    shell::init_shell();

    // 6. Launch the interactive GUI/TUI simulator desktop or CLI fallback
    #[cfg(feature = "gui-window")]
    {
        gui::start_gui_environment()?;
    }

    #[cfg(all(feature = "tui", not(feature = "gui-window")))]
    {
        gui::start_tui_environment()?;
    }

    #[cfg(not(any(feature = "gui-window", feature = "tui")))]
    {
        use std::io::Write;

        // Print the beautiful UEFI boot banner to standard console
        kernel::boot::print_boot_banner();
        kernel::boot::print_uefi_memory_map();
        println!("\n*** RUNNING IN CLI FALLBACK MODE ***");
        println!("Type 'help' to see shell commands. Type 'exit' to quit.\n");

        let active_user = userspace::get_current_user().unwrap().username;
        print!("[{}@novaos]$ ", active_user);
        std::io::stdout().flush()?;

        let stdin = std::io::stdin();
        let mut input = String::new();
        while stdin.read_line(&mut input).is_ok() {
            let cmd = input.trim();
            if cmd == "exit" {
                break;
            }

            let output = shell::execute_command(cmd);
            print!("{}", output);

            let active_user = userspace::get_current_user().unwrap().username;
            print!("[{}@novaos]$ ", active_user);
            std::io::stdout().flush()?;
            input.clear();
        }
    }

    Ok(())
}
