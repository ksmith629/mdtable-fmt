use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut write = false;
    let mut max_width = None;
    let mut path = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--write" {
            write = true;
        } else if arg == "--width" {
            let value = match args.next() {
                Some(value) => value,
                None => {
                    eprintln!("mdtable-fmt: --width needs a number");
                    return ExitCode::FAILURE;
                }
            };
            match value.parse::<usize>() {
                Ok(width) if width > 0 => max_width = Some(width),
                _ => {
                    eprintln!("mdtable-fmt: --width expects a positive number, got {value}");
                    return ExitCode::FAILURE;
                }
            }
        } else if path.is_none() {
            path = Some(arg);
        } else {
            eprintln!("mdtable-fmt: unexpected extra argument {arg}");
            return ExitCode::FAILURE;
        }
    }

    let path = match (write, path) {
        (true, None) => {
            eprintln!("mdtable-fmt: --write needs a file path, stdin can't be edited in place");
            return ExitCode::FAILURE;
        }
        (_, path) => path,
    };

    let input = match &path {
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

    match mdtable_fmt::normalize_document_with_width(&input, max_width) {
        Some(output) => {
            if write {
                let path = path.expect("--write without a path was rejected above");
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
