use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
    process,
};

use rusk::{source_map_json, transpile};

#[derive(Debug, Default)]
struct Args {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    source_map: Option<PathBuf>,
}

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("rusk: {error}");
        process::exit(1);
    }
}

fn run(raw_args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(raw_args)?;
    let source = read_source(args.input.as_ref())?;
    let output = transpile(&source)?;

    if let Some(path) = args.output {
        fs::write(path, output.rust)?;
    } else {
        print!("{}", output.rust);
    }

    if let Some(path) = args.source_map {
        fs::write(path, source_map_json(&output.source_map))?;
    }

    Ok(())
}

fn parse_args(raw_args: Vec<String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut index = 0usize;
    let raw_args = if raw_args.first().is_some_and(|arg| arg == "transpile") {
        raw_args.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        raw_args
    };

    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "-o" | "--output" => {
                index += 1;
                let Some(value) = raw_args.get(index) else {
                    return Err("--output requires a path".to_string());
                };
                args.output = Some(PathBuf::from(value));
            }
            "--source-map" => {
                index += 1;
                let Some(value) = raw_args.get(index) else {
                    return Err("--source-map requires a path".to_string());
                };
                args.source_map = Some(PathBuf::from(value));
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => {
                if args.input.is_some() {
                    return Err("only one input path is supported".to_string());
                }
                args.input = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    Ok(args)
}

fn read_source(path: Option<&PathBuf>) -> Result<String, Box<dyn std::error::Error>> {
    match path {
        Some(path) => Ok(fs::read_to_string(path)?),
        None => {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            Ok(source)
        }
    }
}

fn print_help() {
    println!(
        "rusk - indentation-based Rust syntax transpiler\n\nUSAGE:\n    rusk [transpile] [INPUT] [-o OUTPUT] [--source-map SOURCE_MAP]\n\nIf INPUT is omitted, rusk reads RSML/Rusk source from stdin and writes Rust to stdout."
    );
}
