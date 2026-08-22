use crate::config::LoadedConfig;

pub fn print_config(loaded: &LoadedConfig) {
    println!("LCU Configuration\n");
    println!("Config file");
    if loaded.found {
        println!("  {}\n", loaded.path.display());
    } else {
        println!("  not found ({})\n", loaded.path.display());
        println!("Using default configuration.\n");
    }
    println!("Output");
    println!("  Language:        {}\n", loaded.config.output.language);
    println!("Scanner");
    println!(
        "  Max file size:   {} KB",
        loaded.config.scanner.max_file_size_kb
    );
    println!(
        "  Ignore hidden:   {}\n",
        if loaded.config.scanner.ignore_hidden {
            "yes"
        } else {
            "no"
        }
    );
    println!("Memory");
    println!("  Mode:            {}", loaded.config.memory.mode);
}
