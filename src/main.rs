mod config;

use std::fs;
use std::io::{self, Read, Write};

use config::{Command, Config};
use cppobfuscator::obfuscate;

fn main() {
    if let Err(message) = run() {
        eprintln!("cppobfuscator: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let command = Config::parse()?;
    let config = match command {
        Command::Help => {
            print!("{}", Config::usage());
            return Ok(());
        }
        Command::Version => {
            println!("cppobfuscator {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Command::Run(config) => config,
    };

    let source = read_source(&config)?;
    let output = obfuscate(&source, &config.options).map_err(|error| error.to_string())?;
    write_output(&config, output.as_bytes())
}

fn read_source(config: &Config) -> Result<String, String> {
    if let Some(path) = &config.input {
        return fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()));
    }

    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| format!("failed to read stdin as UTF-8: {error}"))?;
    Ok(source)
}

fn write_output(config: &Config, output: &[u8]) -> Result<(), String> {
    if let Some(path) = &config.output {
        return fs::write(path, output)
            .map_err(|error| format!("failed to write {}: {error}", path.display()));
    }

    io::stdout()
        .lock()
        .write_all(output)
        .map_err(|error| format!("failed to write stdout: {error}"))
}
