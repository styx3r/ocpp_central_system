use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use config::config::Config;

use ftail::Ftail;
use log::LevelFilter;

use clap::Parser;

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

struct TraceLogger {
    log_directory: String,
    config: ftail::Config,
}

impl log::Log for TraceLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() == self.config.level_filter
            && !metadata.target().contains("tungstenite")
            && !metadata.target().contains("reqwest")
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let time = chrono::Local::now()
            .format(&self.config.datetime_format)
            .to_string();

        let today_filename = chrono::Local::now().format("%Y-%m-%d.trace").to_string();
        let path = Path::new(&self.log_directory).join(&today_filename);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .unwrap();

        writeln!(
            file,
            "[{} | {:<5} | {}] | {}",
            time,
            record.level(),
            record.target(),
            record.args()
        )
        .expect("Could not write to trace log!");

        let _ = file.flush();
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

    let cloned = config.log_directory.clone();
    let log_directory = Path::new(&config.log_directory);
    Ftail::new()
        .custom(
            |config| Box::new(CustomLogger { config }) as Box<dyn log::Log + Send + Sync>,
            LevelFilter::Info,
        )
        .custom(
            move |c| {
                Box::new(TraceLogger {
                    log_directory: cloned.clone(),
                    config: c,
                }) as Box<dyn log::Log + Send + Sync>
            },
            LevelFilter::Trace,
        )
        .max_file_size(50)
        .daily_file(log_directory, LevelFilter::Info) // log errors to daily files
        .retention_days(14)
        .init()?; // initialize logger

    ocppcentral_system::run(&config)
}
