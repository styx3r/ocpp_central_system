use std::fs;
use std::path::Path;
use std::{error::Error, time::Duration};

use config::config::Config;

use ftail::Ftail;
use log::LevelFilter;

use clap::Parser;

use fronius::FroniusApi;

use ocpp::{ChargePointStatus, StatusNotificationRequest};

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
        .init()?; // initialize logger

    let mut hooks = OcppHooks::new(FroniusApi::new(&config.fronius));
    ocpp::run(&config, &mut hooks)?;

    Ok(())
}

//-------------------------------------------------------------------------------------------------

struct OcppHooks(FroniusApi);

impl OcppHooks {
    pub fn new(fronius_api: FroniusApi) -> Self {
        Self(fronius_api)
    }
}

impl ocpp::OcppStatusNotificationHook for OcppHooks {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match status_notification.status {
            ChargePointStatus::Charging => {
                self.0
                    .block_battery_for_duration(&Duration::from_hours(12))?;
            },
            ChargePointStatus::SuspendedEV => {
                self.0.fully_unblock_battery()?;
            },
            _ => {}
        }

        Ok(())
    }
}
