mod api;
mod cli;
mod config;
mod pattern;
mod reporter;
mod scanner;

use clap::Parser;
use cli::{Cli, Commands, Format};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;

const DEFAULT_BASELINE: &str = ".aegis.catalog.json";

/// Color output when the target stream is a terminal and NO_COLOR is unset.
fn color_enabled(stream_is_terminal: bool) -> bool {
    stream_is_terminal && std::env::var_os("NO_COLOR").is_none()
}

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

fn catalog_path(config_catalog: Option<&str>, root: &Path) -> PathBuf {
    root.join(config_catalog.unwrap_or(DEFAULT_BASELINE))
}

fn resolve_registered(
    api_url: Option<&str>,
    baseline: Option<&str>,
    config_catalog: Option<&str>,
    root: &Path,
) -> Result<HashSet<String>, String> {
    match (api_url, baseline) {
        (Some(_), Some(_)) => Err("provide either --api or --baseline, not both".to_string()),
        (Some(url), None) => api::fetch_catalog_api(url),
        (None, Some(path)) => api::load_catalog_file(path),
        (None, None) => {
            let default = catalog_path(config_catalog, root);
            api::load_catalog_file(&default.to_string_lossy()).map_err(|e| {
                format!(
                    "{}\nProvide --api <url>, --baseline <file>, or add a '{}' baseline file.",
                    e,
                    default.display()
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
            save,
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

            let save_path: Option<PathBuf> = match save {
                None => None,
                Some(None) => Some(catalog_path(aegis_config.catalog.as_deref(), &root_path)),
                Some(Some(p)) => Some(PathBuf::from(p)),
            };

            if let Some(path) = save_path {
                let catalog = reporter::report_catalog_json(&results);
                std::fs::write(&path, format!("{}\n", catalog))
                    .map_err(|e| format!("Failed to write catalog to '{}': {}", path.display(), e))?;
                println!("Saved catalog to {}", path.display());
            } else {
                match format {
                    Format::Table => println!("{}", reporter::report_table(&results)),
                    Format::Csv => println!("{}", reporter::report_csv(&results)),
                    Format::Json => println!("{}", reporter::report_json(&results)),
                    Format::CatalogJson => {
                        println!("{}", reporter::report_catalog_json(&results))
                    }
                }
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
            let registered = resolve_registered(
                api_url.as_deref(),
                baseline.as_deref(),
                aegis_config.catalog.as_deref(),
                &root_path,
            )
            .map_err(|e| format!("Diff failed: {}", e))?;
            let added = api::unregistered(&results, &registered);
            let gone = api::removed(&results, &registered);

            if added.is_empty() && gone.is_empty() {
                println!("All permissions registered in catalog.");
            } else {
                let color = color_enabled(std::io::stdout().is_terminal());
                println!("{}", reporter::report_diff(&added, &gone, color));
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
            let registered = resolve_registered(
                api_url.as_deref(),
                baseline.as_deref(),
                aegis_config.catalog.as_deref(),
                &root_path,
            )
            .map_err(|e| format!("Lint failed: {}", e))?;
            let added = api::unregistered(&results, &registered);
            let gone = api::removed(&results, &registered);

            if added.is_empty() {
                if gone.is_empty() {
                    println!("All permissions registered.");
                } else {
                    let color = color_enabled(std::io::stdout().is_terminal());
                    println!("{}", reporter::report_diff(&added, &gone, color));
                }
                return Ok(0);
            }

            let color = color_enabled(std::io::stderr().is_terminal());
            eprintln!("{}", reporter::report_diff(&added, &gone, color));
            return Ok(1);
        }
    }

    Ok(0)
}
