mod api_types;
mod digest_auth;
mod fronius_mock;

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
pub use api_types::*;

//-------------------------------------------------------------------------------------------------

static API_VERSION_URI: &str = "/api/status/version";
static LOGIN_URI: &str = "/api/commands/Login";
static TIME_OF_USE_URI: &str = "/api/config/timeofuse";

static GET_POWER_FLOW_REALTIME_DATA: &str = "/solar_api/v1/GetPowerFlowRealtimeData.fcgi";

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

    fn get_power_flow_realtime_data(
        &mut self,
    ) -> Result<PowerFlowRealtimeData, Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

pub struct FroniusApiAdapter {
    digest_auth: DigestAuth,
    fronius_config: Fronius,
}

//-------------------------------------------------------------------------------------------------

impl FroniusApiAdapter {
    #[cfg(test)]
    fn default() -> Self {
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

    fn parse_powerflow_realtime_data(
        &self,
        payload: &str,
    ) -> Result<PowerFlowRealtimeData, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str::<PowerFlowRealtimeData>(payload)?)
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

    fn get_power_flow_realtime_data(
        &mut self,
    ) -> Result<PowerFlowRealtimeData, Box<dyn std::error::Error>> {
        self.parse_powerflow_realtime_data(
            Client::new()
                .get(&format!(
                    "{}{}",
                    self.fronius_config.url,
                    GET_POWER_FLOW_REALTIME_DATA.to_owned()
                ))
                .send()?
                .text()?
                .as_str(),
        )
    }
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_powerflow_realtime_data_response() -> Result<(), Box<dyn std::error::Error>> {
        static POWERFLOW_REALTIME_DATA_RESPONSE: &str = r#"{
   "Body" : {
      "Data" : {
         "Inverters" : {
            "1" : {
               "Battery_Mode" : "normal",
               "DT" : 1,
               "E_Day" : null,
               "E_Total" : 23329247.706388891,
               "E_Year" : null,
               "P" : 378.76364135742188,
               "SOC" : 94.799999999999997
            }
         },
         "SecondaryMeters" : {},
         "Site" : {
            "BackupMode" : false,
            "BatteryStandby" : true,
            "E_Day" : null,
            "E_Total" : 23329247.706388891,
            "E_Year" : null,
            "Meter_Location" : "grid",
            "Mode" : "bidirectional",
            "P_Akku" : 388.62789916992188,
            "P_Grid" : 16.399999999999999,
            "P_Load" : -395.23117675781248,
            "P_PV" : 38.069236755371094,
            "rel_Autonomy" : 95.850529774869088,
            "rel_SelfConsumption" : 100.0
         },
         "Smartloads" : {
            "OhmpilotEcos" : {},
            "Ohmpilots" : {
               "1" : {
                  "P_AC_Total" : 0.0,
                  "State" : "normal",
                  "Temperature" : 63.5
               }
            }
         },
         "Version" : "13"
      }
   },
   "Head" : {
      "RequestArguments" : {},
      "Status" : {
         "Code" : 0,
         "Reason" : "",
         "UserMessage" : ""
      },
      "Timestamp" : "2026-02-06T16:10:32+00:00"
   }
}
"#;

        // NOTE: Just adding this test to get a fast response if changes are made in the
        // api_types.rs
        let default_fronius_api_adapter = FroniusApiAdapter::default();
        default_fronius_api_adapter
            .parse_powerflow_realtime_data(POWERFLOW_REALTIME_DATA_RESPONSE)?;

        Ok(())
    }
}
