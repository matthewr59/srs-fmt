mod card;

use card::parse_line;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

struct Args {
    lenient: bool,
    input: Option<String>,
    output: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut lenient = false;
    let mut input = None;
    let mut output = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-o" | "--output" => {
                output = Some(args.next().ok_or("--output requires a path")?);
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other if !other.starts_with('-') && input.is_none() => {
                input = Some(other.to_string());
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    Ok(Args {
        lenient,
        input,
        output,
    })
}

fn print_help() {
    println!("srs-fmt - normalize spaced repetition scheduling data\n");
    println!("Usage: srs-fmt [--lenient] [-o OUTPUT] [INPUT]\n");
    println!("Reads front/back/interval/ease/due-date rows and writes the canonical");
    println!("tab-separated form. Reads stdin and writes stdout if paths are omitted.\n");
    println!("  --lenient        tolerate mixed delimiters, decimal commas, extra");
    println!("                   whitespace, and a few common date formats");
    println!("  -o, --output     write to this file instead of stdout");
    println!("  -h, --help       print this message");
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("srs-fmt: {e}");
            return ExitCode::FAILURE;
        }
    };

    let reader: Box<dyn BufRead> = match &args.input {
        Some(path) => match File::open(path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("srs-fmt: cannot open {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(BufReader::new(io::stdin())),
    };

    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => match File::create(path) {
            Ok(f) => Box::new(f),
            Err(e) => {
                eprintln!("srs-fmt: cannot create {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(io::stdout()),
    };

    let mut written = 0usize;

    for (i, line) in reader.lines().enumerate() {
        let line_no = i + 1;
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("srs-fmt: line {line_no}: read error: {e}");
                return ExitCode::FAILURE;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        match parse_line(&line, args.lenient) {
            Ok(card) => {
                if writeln!(writer, "{}", card.to_line()).is_err() {
                    eprintln!("srs-fmt: failed writing output");
                    return ExitCode::FAILURE;
                }
                written += 1;
            }
            Err(e) if args.lenient => {
                eprintln!("srs-fmt: line {line_no}: skipped ({e})");
            }
            Err(e) => {
                eprintln!("srs-fmt: line {line_no}: {e}");
                eprintln!("srs-fmt: pass --lenient to salvage what can be parsed");
                return ExitCode::FAILURE;
            }
        }
    }

    if written == 0 {
        eprintln!("srs-fmt: no valid rows produced");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
