use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut write = false;
    let mut path = None;
    for arg in env::args().skip(1) {
        if arg == "--write" {
            write = true;
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

    match mdtable_fmt::normalize_document(&input) {
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
