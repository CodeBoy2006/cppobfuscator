mod config;
mod lexer;
mod obfuscate;

use std::io::{self, Read, Write};

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

    let mut input = String::new();
    if let Some(path) = config.input.as_ref() {
        input = std::fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("Failed to read {}: {err}", path.display());
            std::process::exit(1);
        });
    } else {
        if let Err(err) = io::stdin().read_to_string(&mut input) {
            eprintln!("Failed to read stdin: {err}");
            std::process::exit(1);
        }
    }

    let obfuscate_config = ObfuscateConfig {
        seed: config.seed,
        rename: config.rename,
        minify: config.minify,
        inline: config.inline,
        constlift: config.constlift,
        strip_comments: config.strip_comments,
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
