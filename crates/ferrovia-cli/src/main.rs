use std::env;
use std::fs;
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
    let svg_path = args
        .next()
        .ok_or_else(|| "usage: ferrovia-cli <svg-path> [config-json-path]".to_string())?;
    let config_path = args.next();

    let svg = fs::read_to_string(&svg_path).map_err(|error| error.to_string())?;
    let config = if let Some(config_path) = config_path {
        let raw = fs::read_to_string(config_path).map_err(|error| error.to_string())?;
        serde_json::from_str::<Config>(&raw).map_err(|error| error.to_string())?
    } else {
        Config::default()
    };

    let result = optimize(&svg, &config).map_err(|error| error.to_string())?;
    print!("{}", result.data);
    Ok(())
}
