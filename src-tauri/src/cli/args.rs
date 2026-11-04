use std::ffi::OsString;
use std::path::PathBuf;

pub(super) const COMMANDS: &[&str] = &["spec", "validate", "inspect", "run", "pack"];
pub(super) const HELP: &str = "Liveleaf CLI\n\nUsage: liveleaf-cli <command> [file.ll.md | --stdin] [options]\n\nCommands:\n  spec                       Describe supported syntax and capabilities\n  validate <file>             Check syntax, IDs, visualization and resources (no execution)\n  inspect <file>              Read slides, code cells and resource metadata\n  run <file> --cell <id>      Execute one Python cell using the desktop runtime\n  pack <file> --output <path> Validate and bundle a .llpkg project\n\nOptions:\n  --json                     Write one versioned JSON response, including on failure\n  --stdin                    Read UTF-8 course source from stdin (or use - as the file)\n  --base-dir <directory>     Resource/working directory for stdin (default: current directory)\n  --force                    Replace an existing output package (pack only)\n  -h, --help                 Show help\n  -V, --version              Show version\n\nExit codes: 0 success; 1 validation/execution failure; 2 usage, I/O or setup failure.\n";

#[derive(Default)]
pub(super) struct Args {
    pub command: String,
    pub input: Option<PathBuf>,
    pub stdin: bool,
    pub base_dir: Option<PathBuf>,
    pub cell: Option<String>,
    pub output: Option<PathBuf>,
    pub force: bool,
    pub help: bool,
    pub version: bool,
}

pub(super) fn parse(args: &[OsString]) -> Result<Args, String> {
    let mut result = Args::default();
    let mut args = args.iter();
    let mut positional_only = false;
    while let Some(arg) = args.next() {
        let text = arg.to_str().unwrap_or("");
        if !positional_only && text == "--" {
            positional_only = true;
            continue;
        }
        if !positional_only && text.starts_with('-') && text != "-" {
            let (key, inline) = text
                .split_once('=')
                .map_or((text, None), |(k, v)| (k, Some(OsString::from(v))));
            if [
                "--json",
                "--stdin",
                "--force",
                "--help",
                "-h",
                "--version",
                "-V",
            ]
            .contains(&key)
                && inline.is_some()
            {
                return Err(format!("{key} does not accept a value."));
            }
            match key {
                "--json" => {}
                "--stdin" => result.stdin = true,
                "--force" => result.force = true,
                "--help" | "-h" => result.help = true,
                "--version" | "-V" => result.version = true,
                "--cell" | "--output" | "-o" | "--base-dir" => {
                    let inline_value = inline.is_some();
                    let value = inline
                        .or_else(|| args.next().cloned())
                        .ok_or_else(|| format!("{key} requires a value."))?;
                    if value.is_empty() || (!inline_value && value.to_str().is_some_and(|v| v.starts_with('-'))) {
                        return Err(format!("{key} requires a non-empty value."));
                    }
                    let repeated = match key {
                        "--cell" => result
                            .cell
                            .replace(value.into_string().map_err(|_| "Cell ID must be UTF-8.")?)
                            .is_some(),
                        "--base-dir" => result.base_dir.replace(value.into()).is_some(),
                        _ => result.output.replace(value.into()).is_some(),
                    };
                    if repeated {
                        return Err(format!("{key} was supplied more than once."));
                    }
                }
                _ => return Err(format!("Unknown option: {key}")),
            }
        } else if result.command.is_empty() {
            if !COMMANDS.contains(&text) {
                return Err(format!("Unknown command: {}", arg.to_string_lossy()));
            }
            result.command = text.into();
        } else if text == "-" {
            if result.stdin {
                return Err("stdin was supplied more than once.".into());
            }
            result.stdin = true;
        } else if result.input.replace(arg.into()).is_some() {
            return Err("Only one input file is accepted.".into());
        }
    }
    if result.command.is_empty() && !result.version {
        result.help = true;
    }
    if result.help || result.version {
        return Ok(result);
    }
    if result.command == "spec" {
        if result.input.is_some() || result.stdin || result.base_dir.is_some() {
            return Err("spec does not accept source input.".into());
        }
    } else if result.stdin == result.input.is_some() {
        return Err("Supply exactly one input file or --stdin.".into());
    }
    if result.base_dir.is_some() && !result.stdin {
        return Err("--base-dir is only valid with stdin input.".into());
    }
    if (result.command == "run") != result.cell.is_some() {
        return Err("run requires --cell <id>; --cell is only valid for run.".into());
    }
    if (result.command == "pack") != result.output.is_some() {
        return Err("pack requires --output <path>; --output is only valid for pack.".into());
    }
    if result.force && result.command != "pack" {
        return Err("--force is only valid for pack.".into());
    }
    Ok(result)
}
