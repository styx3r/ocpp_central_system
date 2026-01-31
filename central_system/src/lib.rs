mod hooks;

use config::config::Config;
use fronius::FroniusApi;
use hooks::OcppHooks;
use log::info;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

//-------------------------------------------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

//-------------------------------------------------------------------------------------------------

pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    info!("Starting OCPPCentralSystem v{}", VERSION);
    let hooks = Arc::new(Mutex::new(OcppHooks::new(
        FroniusApi::new(&config.fronius),
        config.clone(),
    )));

    ocpp::run::<OcppHooks>(&config, Arc::clone(&hooks))
}
