mod cli;
mod config;
mod pattern;
mod reporter;
mod scanner;
mod api;

use clap::Parser;
use cli::{Cli, Commands, Format};
use std::path::PathBuf;
use std::process;

fn main() {
    let cli = Cli::parse();

    match run(cli) {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(2);
        }
    }
}

fn run(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            config,
            root,
            format,
            ignore_rules,
        } => {
            let root_path = root.map(PathBuf::from).unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });

            let (_, aegis_config) = match config {
                Some(ref path) => {
                    let p = PathBuf::from(path);
                    (p.clone(), config::parse_config(&p)?)
                }
                None => config::discover_config(&root_path)?,
            };

            let results = scanner::scan(&aegis_config, &root_path, &ignore_rules);

            match format {
                Format::Table => println!("{}", reporter::report_table(&results)),
                Format::Csv => println!("{}", reporter::report_csv(&results)),
                Format::Json => println!("{}", reporter::report_json(&results)),
                Format::CatalogJson => println!("{}", reporter::report_catalog_json(&results)),
            }
        }

        Commands::Diff { api, config, root } => {
            let root_path = root.map(PathBuf::from).unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });

            let (_, aegis_config) = match config {
                Some(ref path) => {
                    let p = PathBuf::from(path);
                    (p.clone(), config::parse_config(&p)?)
                }
                None => config::discover_config(&root_path)?,
            };

            let results = scanner::scan(&aegis_config, &root_path, &[]);
            let unregistered = api::diff_against_catalog(&results, &api)
                .map_err(|e| format!("Diff failed: {}", e))?;

            if unregistered.is_empty() {
                println!("All permissions registered in catalog.");
            } else {
                println!(
                    "{} unregistered permission(s):\n{}",
                    unregistered.len(),
                    reporter::report_table(&unregistered)
                );
            }
        }

        Commands::Lint {
            api,
            config,
            root,
            ignore_rules,
        } => {
            let root_path = root.map(PathBuf::from).unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });

            let (_, aegis_config) = match config {
                Some(ref path) => {
                    let p = PathBuf::from(path);
                    (p.clone(), config::parse_config(&p)?)
                }
                None => config::discover_config(&root_path)?,
            };

            let results = scanner::scan(&aegis_config, &root_path, &ignore_rules);
            let unregistered = api::diff_against_catalog(&results, &api)
                .map_err(|e| format!("Lint failed: {}", e))?;

            if unregistered.is_empty() {
                println!("All permissions registered.");
                return Ok(0);
            }

            eprintln!(
                "{} unregistered permission(s):\n{}",
                unregistered.len(),
                reporter::report_table(&unregistered)
            );
            return Ok(1);
        }
    }

    Ok(0)
}
