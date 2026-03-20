use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use ferrovia_core::{Config, optimize};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input = None;
    let mut output_path = None;
    let mut config_path = None;
    let mut positional = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--string" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--string expects an SVG string".to_string())?;
                input = Some(Input::Literal(value));
            }
            "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--config expects a JSON file path".to_string())?;
                config_path = Some(value);
            }
            "-o" | "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "-o expects an output path or -".to_string())?;
                output_path = Some(value);
            }
            _ => positional.push(arg),
        }
    }

    if input.is_none() {
        if let Some(first) = positional.first() {
            input = Some(if first == "-" {
                Input::Stdin
            } else {
                Input::File(first.clone())
            });
        }
        if positional.len() > 1 && config_path.is_none() {
            config_path = positional.get(1).cloned();
        }
    }

    let input = input.ok_or_else(|| {
        "usage: ferrovia-cli <svg-path|-> [config-json-path] | ferrovia-cli --string <svg> [--config path] [-o path|-]"
            .to_string()
    })?;

    let svg = match input {
        Input::File(path) => fs::read_to_string(path).map_err(|error| error.to_string())?,
        Input::Literal(svg) => svg,
        Input::Stdin => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|error| error.to_string())?;
            buffer
        }
    };

    let config = if let Some(config_path) = config_path {
        let raw = fs::read_to_string(config_path).map_err(|error| error.to_string())?;
        serde_json::from_str::<Config>(&raw).map_err(|error| error.to_string())?
    } else {
        Config::default()
    };

    let result = optimize(&svg, &config).map_err(|error| error.to_string())?;
    match output_path.as_deref() {
        Some("-") | None => print!("{}", result.data),
        Some(path) => fs::write(path, result.data).map_err(|error| error.to_string())?,
    }
    Ok(())
}

enum Input {
    File(String),
    Literal(String),
    Stdin,
}
