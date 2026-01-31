use std::fs;
use std::path::Path;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use config::config::Config;

use ftail::Ftail;
use log::{LevelFilter, info};

use clap::Parser;

mod hooks;

use fronius::FroniusApi;
use hooks::OcppHooks;

//-------------------------------------------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

//-------------------------------------------------------------------------------------------------

/// OCPP central management system.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the config file.
    #[arg(short, long)]
    config_path: String,
}

//-------------------------------------------------------------------------------------------------

struct CustomLogger {
    config: ftail::Config,
}

impl log::Log for CustomLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.config.level_filter
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let time = chrono::Local::now()
            .format(&self.config.datetime_format)
            .to_string();
        println!("{} {:<5} | {}", time, record.level(), record.args());
    }

    fn flush(&self) {}
}

//-------------------------------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let contents = match fs::read_to_string(&args.config_path) {
        // If successful return the files text as `contents`.
        // `c` is a local variable.
        Ok(c) => c,
        // Handle the `error` case.
        Err(_) => {
            // Write `msg` to `stderr`.
            panic!("Could not read file `{}`", args.config_path);
        }
    };

    let config: Config = match toml::from_str(&contents) {
        // If successful, return data as `Data` struct.
        // `d` is a local variable.
        Ok(d) => d,
        // Handle the `error` case.
        Err(_) => {
            // Write `msg` to `stderr`.
            panic!("Unable to load data from `{}`", args.config_path);
        }
    };

    Ftail::new()
        .custom(
            |config| Box::new(CustomLogger { config }) as Box<dyn log::Log + Send + Sync>,
            LevelFilter::Info,
        )
        .max_file_size(50)
        .daily_file(Path::new(&config.log_directory), LevelFilter::Info) // log errors to daily files
        .single_file(Path::new(&config.log_directory), true, LevelFilter::Trace)
        .retention_days(14)
        .init()?; // initialize logger

    info!("Starting OCPPCentralSystem v{}", VERSION);

    let hooks = Arc::new(Mutex::new(OcppHooks::new(
        FroniusApi::new(&config.fronius),
        config.clone(),
    )));

    ocpp::run::<OcppHooks>(&config, Arc::clone(&hooks))?;

    Ok(())
}
