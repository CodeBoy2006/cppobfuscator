mod config;
mod lexer;
mod obfuscate;
mod semantics;

use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};

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

    let mut input = if config.wizard {
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

    if config.expand_macros {
        input = expand_macros(&input).unwrap_or_else(|err| {
            eprintln!("{err}");
            std::process::exit(1);
        });
    }

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

fn expand_macros(input: &str) -> Result<String, String> {
    let mut includes = Vec::new();
    let mut pre_input = String::new();

    for line in input.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#include") {
            let marker = format!("/*__CPP_OBFUSCATOR_INCLUDE_{}__*/", includes.len());
            includes.push(format!("{line}\n"));
            pre_input.push_str(&marker);
            pre_input.push('\n');
        } else {
            pre_input.push_str(line);
            pre_input.push('\n');
        }
    }

    let mut child = Command::new("g++-15")
        .args(["-E", "-P", "-CC", "-x", "c++", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run g++-15 for macro expansion: {err}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(pre_input.as_bytes())
            .map_err(|err| format!("Failed to send input to g++-15: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to read g++-15 output: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "g++-15 macro expansion failed: {}",
            stderr.trim()
        ));
    }

    let mut expanded = String::from_utf8(output.stdout)
        .map_err(|err| format!("g++-15 output was not valid UTF-8: {err}"))?;

    for (idx, include) in includes.iter().enumerate() {
        let marker = format!("/*__CPP_OBFUSCATOR_INCLUDE_{}__*/", idx);
        expanded = expanded.replace(&marker, include);
    }

    Ok(expanded)
}
