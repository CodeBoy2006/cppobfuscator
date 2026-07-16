use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;

use cppobfuscator::Options;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Config {
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub options: Options,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Run(Config),
    Help,
    Version,
}

impl Config {
    pub(crate) fn parse() -> Result<Command, String> {
        Self::parse_args(env::args().skip(1))
    }

    fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
        let mut input = None;
        let mut output = None;
        let mut seed = Options::default().seed;
        let mut compact = true;
        let mut strip_comments = true;
        let mut preserve = BTreeSet::new();
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "-h" | "--help" => return Ok(Command::Help),
                "-V" | "--version" => return Ok(Command::Version),
                "-i" | "--input" => {
                    input = Some(PathBuf::from(required_value(&mut args, "--input")?));
                }
                "-o" | "--output" => {
                    output = Some(PathBuf::from(required_value(&mut args, "--output")?));
                }
                "--seed" => {
                    let value = required_value(&mut args, "--seed")?;
                    seed = parse_seed(&value)?;
                }
                "--preserve" => {
                    let name = required_value(&mut args, "--preserve")?;
                    if !is_identifier(&name) {
                        return Err(format!("--preserve expects a C++ identifier, got {name:?}"));
                    }
                    preserve.insert(name);
                }
                "--keep-comments" => strip_comments = false,
                "--keep-layout" => compact = false,
                _ => {
                    return Err(format!("unknown argument: {argument}\n\n{}", Self::usage()));
                }
            }
        }

        Ok(Command::Run(Self {
            input,
            output,
            options: Options {
                seed,
                compact,
                strip_comments,
                preserve,
            },
        }))
    }

    pub(crate) fn usage() -> &'static str {
        r#"cppobfuscator - Tree-sitter based C++ single-file obfuscator

USAGE:
  cppobfuscator [OPTIONS]

OPTIONS:
  -i, --input <PATH>       Read C++ source from a file (default: stdin)
  -o, --output <PATH>      Write transformed source to a file (default: stdout)
      --seed <U64>         Rename seed; decimal or 0x-prefixed (default: 0xC0FFEE)
      --preserve <NAME>    Preserve an identifier; may be repeated
      --keep-comments      Keep comments
      --keep-layout        Keep original whitespace and line layout
  -h, --help               Print help
  -V, --version            Print version
"#
    }
}

fn required_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_seed(value: &str) -> Result<u64, String> {
    let parsed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(|| value.parse::<u64>(), |hex| u64::from_str_radix(hex, 16));
    parsed.map_err(|_| format!("invalid --seed value: {value:?}"))
}

fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Command, String> {
        Config::parse_args(arguments.iter().map(|argument| (*argument).to_string()))
    }

    #[test]
    fn parses_ast_options() {
        let command = parse(&[
            "--seed",
            "0x2a",
            "--keep-comments",
            "--keep-layout",
            "--preserve",
            "solve",
        ])
        .unwrap();
        let Command::Run(config) = command else {
            panic!("expected run command");
        };

        assert_eq!(config.options.seed, 42);
        assert!(!config.options.compact);
        assert!(!config.options.strip_comments);
        assert!(config.options.preserve.contains("solve"));
    }

    #[test]
    fn rejects_removed_legacy_options() {
        let error = parse(&["--no-inline"]).unwrap_err();
        assert!(error.contains("unknown argument"));
    }

    #[test]
    fn validates_preserved_identifiers() {
        let error = parse(&["--preserve", "not-a-name"]).unwrap_err();
        assert!(error.contains("C++ identifier"));
    }
}
