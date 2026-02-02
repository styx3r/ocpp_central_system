mod api_types;
mod digest_auth;
mod fronius_mock;

use api_types::*;
use config::config::Fronius;
use digest_auth::DigestAuth;

use chrono::{Datelike, Weekday, offset::Local};
use log::info;
use reqwest::{
    StatusCode,
    blocking::{Client, Response},
};

use std::time::Duration;

pub use fronius_mock::FroniusMock;

//-------------------------------------------------------------------------------------------------

static API_VERSION_URI: &str = "/api/status/version";
static LOGIN_URI: &str = "/api/commands/Login";
static TIME_OF_USE_URI: &str = "/api/config/timeofuse";

static SUPPORTED_COMMANDS_API_VERSION: &str = "8.4.1";
static SUPPORTED_CONFIG_API_VERSION: &str = "10.2.0";

static TIMES_OF_USE_WRITE_SUCCESS: &str = "timeofuse";

static TIME_TABLE_FORMAT: &str = "%H:%M";

//-------------------------------------------------------------------------------------------------

pub trait FroniusApi {
    fn block_battery_for_duration(
        &mut self,
        duration_to_block: &Duration,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn fully_unblock_battery(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

pub struct FroniusApiAdapter {
    digest_auth: DigestAuth,
    fronius_config: Fronius,
}

//-------------------------------------------------------------------------------------------------

impl FroniusApiAdapter {
    pub fn default() -> Self {
        Self {
            digest_auth: DigestAuth::new(&"".to_owned(), &"".to_owned()),
            fronius_config: Fronius {
                username: "".to_owned(),
                password: "".to_owned(),
                url: "".to_owned(),
            },
        }
    }

    pub fn new(fronius_config: &Fronius) -> Result<Self, Box<dyn std::error::Error>> {
        let self_ = Self {
            digest_auth: DigestAuth::new(&fronius_config.username, &fronius_config.password),
            fronius_config: fronius_config.clone(),
        };

        match self_.check_firmware_version() {
            Ok(_) => Ok(self_),
            Err(e) => Err(e),
        }
    }

    fn check_firmware_version(&self) -> Result<(), Box<dyn std::error::Error>> {
        let firmware_status_url = format!("{}{}", &self.fronius_config.url, API_VERSION_URI);
        let response: Response = Client::new().get(&firmware_status_url).send()?;

        if response.status() != StatusCode::OK {
            return Err("Could not query Fronius API version!".into());
        }

        let api_version_response =
            serde_json::from_str::<ApiVersionResponse>(response.text()?.as_str())?;

        if api_version_response.api_versions.commands_api
            != SUPPORTED_COMMANDS_API_VERSION.to_owned()
            || api_version_response.api_versions.config_api
                != SUPPORTED_CONFIG_API_VERSION.to_owned()
        {
            return Err(format!(
                "Commands API version ({}) OR config API version({}) does not match supported API versions ({}, {})!",
                api_version_response.api_versions.commands_api,
                api_version_response.api_versions.config_api,
                SUPPORTED_COMMANDS_API_VERSION,
                SUPPORTED_CONFIG_API_VERSION
            )
            .into());
        }

        Ok(())
    }

    fn login(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.check_firmware_version()?;

        let login_respone = serde_json::from_str::<LoginResponse>(
            self.digest_auth
                .get(
                    &self.fronius_config.url,
                    &LOGIN_URI.to_owned(),
                    &format!("user={}", &self.fronius_config.username),
                )?
                .as_str(),
        )?;

        if login_respone.success {
            Ok(())
        } else {
            Err("Login failed".into())
        }
    }
}

impl FroniusApi for FroniusApiAdapter {
    fn block_battery_for_duration(
        &mut self,
        duration_to_block: &Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Early login because if login does not work nothing else SHALL happen!
        self.login()?;

        let start = Local::now();
        let end = start + *duration_to_block;

        let formatted_start = start.format(TIME_TABLE_FORMAT);
        let formatted_end = end.format(TIME_TABLE_FORMAT);

        let weekdays = Weekdays {
            monday: start.weekday() == Weekday::Mon || end.weekday() == Weekday::Mon,
            tuesday: start.weekday() == Weekday::Tue || end.weekday() == Weekday::Tue,
            wednesday: start.weekday() == Weekday::Wed || end.weekday() == Weekday::Wed,
            thursday: start.weekday() == Weekday::Thu || end.weekday() == Weekday::Thu,
            friday: start.weekday() == Weekday::Fri || end.weekday() == Weekday::Fri,
            saturday: start.weekday() == Weekday::Sat || end.weekday() == Weekday::Sat,
            sunday: start.weekday() == Weekday::Sun || end.weekday() == Weekday::Sun,
        };

        let schedule_type = ScheduleType::DischargeMax;

        let times_of_use = TimesOfUse {
            time_of_use: if start.weekday() == end.weekday() {
                vec![TimeOfUse {
                    active: true,
                    power: 0,
                    schedule_type,
                    time_table: TimeTable {
                        start: formatted_start.to_string(),
                        end: formatted_end.to_string(),
                    },
                    weekdays,
                }]
            } else {
                vec![
                    TimeOfUse {
                        active: true,
                        power: 0,
                        schedule_type,
                        time_table: TimeTable {
                            start: formatted_start.to_string(),
                            end: "23:59".into(),
                        },
                        weekdays: weekdays,
                    },
                    TimeOfUse {
                        active: true,
                        power: 0,
                        schedule_type,
                        time_table: TimeTable {
                            start: "00:00".into(),
                            end: formatted_end.to_string(),
                        },
                        weekdays,
                    },
                ]
            },
        };

        let response = serde_json::from_str::<TimesOfUseResponse>(
            self.digest_auth
                .post_json(
                    &self.fronius_config.url,
                    &TIME_OF_USE_URI.to_owned(),
                    &times_of_use,
                )?
                .as_str(),
        )?;

        if !response
            .write_success
            .contains(&TIMES_OF_USE_WRITE_SUCCESS.into())
        {
            return Err("Could not block Battery!".into());
        }

        info!("Blocked battery!");
        Ok(())
    }

    fn fully_unblock_battery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.login()?;

        let empty_time_of_use = TimesOfUse {
            time_of_use: vec![],
        };
        let response = serde_json::from_str::<TimesOfUseResponse>(
            self.digest_auth
                .post_json(
                    &self.fronius_config.url,
                    &TIME_OF_USE_URI.to_owned(),
                    &empty_time_of_use,
                )?
                .as_str(),
        )?;

        if !response
            .write_success
            .contains(&TIMES_OF_USE_WRITE_SUCCESS.into())
        {
            return Err("Could not un-block Battery!".into());
        }

        info!("Battery unblocked!");
        Ok(())
    }
}
