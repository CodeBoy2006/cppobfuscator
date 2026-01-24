mod config;
mod lexer;
mod obfuscate;
mod semantics;

use std::io::{self, BufRead, Read, Write};

use config::Config;
use obfuscate::{obfuscate, ObfuscateConfig};

fn main() {
    let config = match Config::parse() {
        Ok(cfg) => cfg,
        Err(msg) => {
            if msg == Config::usage() {
                println!("{msg}");
                return;
            }
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    let input = if config.wizard {
        read_wizard_input(&config.wizard_end)
    } else if let Some(path) = config.input.as_ref() {
        std::fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("Failed to read {}: {err}", path.display());
            std::process::exit(1);
        })
    } else {
        let mut buffer = String::new();
        if let Err(err) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Failed to read stdin: {err}");
            std::process::exit(1);
        }
        buffer
    };

    let obfuscate_config = ObfuscateConfig {
        seed: config.seed,
        rename: config.rename,
        minify: config.minify,
        inline: config.inline,
        constlift: config.constlift,
        strip_comments: config.strip_comments,
        strip_unused_macros: config.strip_unused_macros,
        strip_unused_functions: config.strip_unused_functions,
        preserve: config.preserve,
    };

    let output = obfuscate(&input, &obfuscate_config);

    if let Some(path) = config.output.as_ref() {
        if let Err(err) = std::fs::write(path, output) {
            eprintln!("Failed to write {}: {err}", path.display());
            std::process::exit(1);
        }
    } else {
        let mut stdout = io::stdout();
        if let Err(err) = stdout.write_all(output.as_bytes()) {
            eprintln!("Failed to write stdout: {err}");
            std::process::exit(1);
        }
    }
}

fn read_wizard_input(marker: &str) -> String {
    eprintln!("Wizard mode: paste C++ code, end with line: {marker}");
    eprintln!("Press Ctrl-D to finish early.");

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut input = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = handle.read_line(&mut line).unwrap_or_else(|err| {
            eprintln!("Failed to read stdin: {err}");
            std::process::exit(1);
        });
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
        if trimmed == marker {
            break;
        }
        input.push_str(&line);
    }

    input
}
