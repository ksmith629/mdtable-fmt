use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

struct Config {
    write: bool,
    max_width: Option<usize>,
    path: Option<String>,
}

/// Parses CLI arguments into a `Config`, or an error message (without the
/// `mdtable-fmt: ` prefix `main` adds) describing what was wrong. Pulled out
/// of `main` so the parsing logic can be tested without touching real files
/// or stdio.
fn parse_args<I: Iterator<Item = String>>(args: I) -> Result<Config, String> {
    let mut write = false;
    let mut max_width = None;
    let mut path = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--write" {
            write = true;
        } else if arg == "--width" {
            let value = args.next().ok_or("--width needs a number")?;
            match value.parse::<usize>() {
                Ok(width) if width > 0 => max_width = Some(width),
                _ => return Err(format!("--width expects a positive number, got {value}")),
            }
        } else if path.is_none() {
            path = Some(arg);
        } else {
            return Err(format!("unexpected extra argument {arg}"));
        }
    }

    if write && path.is_none() {
        return Err("--write needs a file path, stdin can't be edited in place".to_string());
    }

    Ok(Config {
        write,
        max_width,
        path,
    })
}

fn main() -> ExitCode {
    let config = match parse_args(env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("mdtable-fmt: {err}");
            return ExitCode::FAILURE;
        }
    };

    let input = match &config.path {
        Some(path) => match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) => {
                eprintln!("mdtable-fmt: couldn't read {path}: {err}");
                return ExitCode::FAILURE;
            }
        },
        None => {
            let mut buf = String::new();
            if let Err(err) = io::stdin().read_to_string(&mut buf) {
                eprintln!("mdtable-fmt: couldn't read stdin: {err}");
                return ExitCode::FAILURE;
            }
            buf
        }
    };

    match mdtable_fmt::normalize_document_with_width(&input, config.max_width) {
        Some(output) => {
            if config.write {
                let path = config.path.expect("--write without a path was rejected above");
                if let Err(err) = fs::write(&path, &output) {
                    eprintln!("mdtable-fmt: couldn't write {path}: {err}");
                    return ExitCode::FAILURE;
                }
            } else {
                print!("{output}");
            }
            ExitCode::SUCCESS
        }
        None => {
            eprintln!(
                "mdtable-fmt: no markdown table found (need a header row followed by a `---` separator row)"
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter()
    }

    #[test]
    fn parse_args_defaults_with_no_arguments() {
        let config = parse_args(args(&[])).unwrap();
        assert!(!config.write);
        assert_eq!(config.max_width, None);
        assert_eq!(config.path, None);
    }

    #[test]
    fn parse_args_reads_a_bare_path() {
        let config = parse_args(args(&["notes.md"])).unwrap();
        assert_eq!(config.path, Some("notes.md".to_string()));
    }

    #[test]
    fn parse_args_sets_write_flag() {
        let config = parse_args(args(&["--write", "notes.md"])).unwrap();
        assert!(config.write);
        assert_eq!(config.path, Some("notes.md".to_string()));
    }

    #[test]
    fn parse_args_write_without_path_is_an_error() {
        let err = parse_args(args(&["--write"])).unwrap_err();
        assert!(err.contains("--write needs a file path"));
    }

    #[test]
    fn parse_args_reads_width() {
        let config = parse_args(args(&["--width", "20", "notes.md"])).unwrap();
        assert_eq!(config.max_width, Some(20));
    }

    #[test]
    fn parse_args_width_missing_value_is_an_error() {
        let err = parse_args(args(&["--width"])).unwrap_err();
        assert!(err.contains("--width needs a number"));
    }

    #[test]
    fn parse_args_width_zero_is_an_error() {
        let err = parse_args(args(&["--width", "0"])).unwrap_err();
        assert!(err.contains("--width expects a positive number"));
    }

    #[test]
    fn parse_args_width_non_numeric_is_an_error() {
        let err = parse_args(args(&["--width", "wide"])).unwrap_err();
        assert!(err.contains("--width expects a positive number, got wide"));
    }

    #[test]
    fn parse_args_rejects_a_second_path() {
        let err = parse_args(args(&["a.md", "b.md"])).unwrap_err();
        assert!(err.contains("unexpected extra argument b.md"));
    }

    #[test]
    fn parse_args_order_of_flags_does_not_matter() {
        let config = parse_args(args(&["notes.md", "--width", "10", "--write"])).unwrap();
        assert!(config.write);
        assert_eq!(config.max_width, Some(10));
        assert_eq!(config.path, Some("notes.md".to_string()));
    }
}
