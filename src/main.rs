use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = env::args().nth(1);

    let input = match path {
        Some(path) => match fs::read_to_string(&path) {
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
            print!("{output}");
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
