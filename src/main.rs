mod api;
mod cli;
mod config;
mod pattern;
mod reporter;
mod scanner;

use clap::Parser;
use cli::{Cli, Commands, Format};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process;

const DEFAULT_BASELINE: &str = ".aegis.catalog.json";

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

fn resolve_registered(
    api_url: Option<&str>,
    baseline: Option<&str>,
    root: &Path,
) -> Result<HashSet<String>, String> {
    match (api_url, baseline) {
        (Some(_), Some(_)) => Err("provide either --api or --baseline, not both".to_string()),
        (Some(url), None) => api::fetch_catalog_api(url),
        (None, Some(path)) => api::load_catalog_file(path),
        (None, None) => {
            let default = root.join(DEFAULT_BASELINE);
            api::load_catalog_file(&default.to_string_lossy()).map_err(|e| {
                format!(
                    "{}\nProvide --api <url>, --baseline <file>, or add a '{}' baseline file.",
                    e, DEFAULT_BASELINE
                )
            })
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
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

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

        Commands::Diff {
            api: api_url,
            baseline,
            config,
            root,
        } => {
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let (_, aegis_config) = match config {
                Some(ref path) => {
                    let p = PathBuf::from(path);
                    (p.clone(), config::parse_config(&p)?)
                }
                None => config::discover_config(&root_path)?,
            };

            let results = scanner::scan(&aegis_config, &root_path, &[]);
            let registered =
                resolve_registered(api_url.as_deref(), baseline.as_deref(), &root_path)
                    .map_err(|e| format!("Diff failed: {}", e))?;
            let missing = api::unregistered(&results, &registered);

            if missing.is_empty() {
                println!("All permissions registered in catalog.");
            } else {
                println!(
                    "{} unregistered permission(s):\n{}",
                    missing.len(),
                    reporter::report_table(&missing)
                );
            }
        }

        Commands::Lint {
            api: api_url,
            baseline,
            config,
            root,
            ignore_rules,
        } => {
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let (_, aegis_config) = match config {
                Some(ref path) => {
                    let p = PathBuf::from(path);
                    (p.clone(), config::parse_config(&p)?)
                }
                None => config::discover_config(&root_path)?,
            };

            let results = scanner::scan(&aegis_config, &root_path, &ignore_rules);
            let registered =
                resolve_registered(api_url.as_deref(), baseline.as_deref(), &root_path)
                    .map_err(|e| format!("Lint failed: {}", e))?;
            let missing = api::unregistered(&results, &registered);

            if missing.is_empty() {
                println!("All permissions registered.");
                return Ok(0);
            }

            eprintln!(
                "{} unregistered permission(s):\n{}",
                missing.len(),
                reporter::report_table(&missing)
            );
            return Ok(1);
        }
    }

    Ok(0)
}
