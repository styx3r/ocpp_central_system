pub mod hooks;

use awattar::AwattarApiAdapter;
use config::config::Config;
use fronius::FroniusApiAdapter;
use hooks::OcppHooks;
use log::info;
use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
};

//-------------------------------------------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const GIT_COMMIT_HASH: &'static str = env!("GIT_COMMIT_HASH");

//-------------------------------------------------------------------------------------------------

/// Main entry point. Basically only a wrapper to enable integration tests
pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    info!("Starting OCPPCentralSystem v{} - {}", VERSION, GIT_COMMIT_HASH);
    let hooks = Arc::new(Mutex::new(OcppHooks::new(
        Arc::new(Mutex::new(FroniusApiAdapter::new(&config.fronius)?)),
        Arc::new(Mutex::new(AwattarApiAdapter::default())),
        config.clone(),
    )));

    ocpp::run::<OcppHooks<FroniusApiAdapter, AwattarApiAdapter>>(&config, Arc::clone(&hooks))
}
