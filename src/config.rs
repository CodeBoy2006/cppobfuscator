use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub seed: u64,
    pub rename: bool,
    pub minify: bool,
    pub inline: bool,
    pub constlift: bool,
    pub strip_comments: bool,
    pub preserve: Vec<String>,
}

impl Config {
    pub fn parse() -> Result<Self, String> {
        let mut input = None;
        let mut output = None;
        let mut seed = 0xC0FFEE_u64;
        let mut rename = true;
        let mut minify = true;
        let mut inline = true;
        let mut constlift = true;
        let mut strip_comments = true;
        let mut preserve = Vec::new();

        let mut args = env::args().skip(1).peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => return Err(Self::usage()),
                "-i" | "--input" => {
                    let path = args.next().ok_or("--input requires a path")?;
                    input = Some(PathBuf::from(path));
                }
                "-o" | "--output" => {
                    let path = args.next().ok_or("--output requires a path")?;
                    output = Some(PathBuf::from(path));
                }
                "--seed" => {
                    let raw = args.next().ok_or("--seed requires a value")?;
                    seed = raw
                        .parse::<u64>()
                        .map_err(|_| "--seed must be an unsigned integer")?;
                }
                "--no-rename" => rename = false,
                "--no-minify" => minify = false,
                "--no-inline" => inline = false,
                "--no-constlift" => constlift = false,
                "--keep-comments" => strip_comments = false,
                "--preserve" => {
                    let raw = args.next().ok_or("--preserve requires a name")?;
                    preserve.push(raw);
                }
                _ => return Err(format!("Unknown argument: {arg}\n\n{}", Self::usage())),
            }
        }

        Ok(Self {
            input,
            output,
            seed,
            rename,
            minify,
            inline,
            constlift,
            strip_comments,
            preserve,
        })
    }

    pub fn usage() -> String {
        let text = r#"cppobfuscator - C++ single-file obfuscator

USAGE:
  cppobfuscator [options]

OPTIONS:
  -i, --input <path>        Input C++ file (defaults to stdin)
  -o, --output <path>       Output file (defaults to stdout)
  --seed <u64>              Seed for deterministic renaming (default: 0xC0FFEE)
  --no-rename               Disable identifier renaming
  --no-minify               Preserve original whitespace/newlines
  --no-inline               Disable inline substitution for simple functions
  --no-constlift            Disable constant expression lifting
  --keep-comments           Preserve comments (default strips)
  --preserve <name>         Preserve an identifier (repeatable)
  -h, --help                Show this help
"#;
        text.to_string()
    }
}
