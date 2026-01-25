mod digest_auth;

use config::config::Fronius;
use digest_auth::DigestAuth;

use chrono::{Datelike, Weekday, offset::Local};
use reqwest::{
    StatusCode,
    blocking::{Client, Response},
};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------

static API_VERSION_URI: &str = "/api/status/version";
static LOGIN_URI: &str = "/api/commands/Login";
static TIME_OF_USE_URI: &str = "/api/config/timeofuse";

static SUPPORTED_COMMANDS_API_VERSION: &str = "8.4.1";
static SUPPORTED_CONFIG_API_VERSION: &str = "10.2.0";

static TIMES_OF_USE_WRITE_SUCCESS: &str = "timeofuse";

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct ResultData {
    pub roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct LoginResponse {
    #[serde(alias = "resultData")]
    pub result_data: ResultData,
    pub success: bool,
}

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct ApiVersions {
    #[serde(alias = "ComponentsApi")]
    components_api: String,
    #[serde(alias = "commandsApi")]
    commands_api: String,
    #[serde(alias = "configApi")]
    config_api: String,
    #[serde(alias = "setupAppApi")]
    setup_app_api: String,
    #[serde(alias = "setupAppUpdateApi")]
    setup_app_update_api: String,
    #[serde(alias = "statusApi")]
    status_api: String,
    #[serde(alias = "updateApi")]
    update_api: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct AppVersions {
    #[serde(alias = "minAndroidVersion")]
    min_android_version: usize,
    #[serde(alias = "minIOSVersion")]
    min_ios_version: usize,
    #[serde(alias = "minWinVersion")]
    min_win_version: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct ApiVersionResponse {
    #[serde(alias = "apiversions")]
    api_versions: ApiVersions,
    #[serde(alias = "articleNumber")]
    article_number: String,
    #[serde(alias = "commonName")]
    common_name: String,
    #[serde(alias = "devicegroup")]
    device_group: String,
    #[serde(alias = "devicename")]
    device_name: String,
    #[serde(alias = "hardwareId")]
    hardware_id: String,
    #[serde(alias = "hwrevisions")]
    hardware_revisions: HashMap<String, String>,
    #[serde(alias = "minAppVersions")]
    min_app_versions: AppVersions,
    #[serde(alias = "numberOfPhases")]
    number_of_phases: usize,
    #[serde(alias = "serialNumber")]
    serial_number: String,
    #[serde(alias = "softwareVersionPrefix")]
    software_version_prefix: String,
    #[serde(alias = "swrevisions")]
    software_revisions: HashMap<String, String>,
}

/*
 * {
 *   "timeofuse": [
 *     {
 *       "Active": true,
 *       "Power": 0,
 *       "ScheduleType": "DISCHARGE_MAX",
 *       "TimeTable": {
 *         "Start": "20:00",
 *         "End": "23:00"
 *       },
 *       "Weekdays": {
 *         "Mon": false,
 *         "Tue": false,
 *         "Wed": false,
 *         "Thu": false,
 *         "Fri": true,
 *         "Sat": false,
 *         "Sun": false
 *       }
 *     }
 *   ]
 * }
 */

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TimeTable {
    #[serde(rename = "Start")]
    start: String,
    #[serde(rename = "End")]
    end: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
struct Weekdays {
    #[serde(rename = "Mon")]
    monday: bool,
    #[serde(rename = "Tue")]
    tuesday: bool,
    #[serde(rename = "Wed")]
    wednesday: bool,
    #[serde(rename = "Thu")]
    thursday: bool,
    #[serde(rename = "Fri")]
    friday: bool,
    #[serde(rename = "Sat")]
    saturday: bool,
    #[serde(rename = "Sun")]
    sunday: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
enum ScheduleType {
    #[serde(rename = "DISCHARGE_MAX")]
    DischargeMax,
    #[serde(rename = "DISCHARGE_MIN")]
    DischargeMin,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TimeOfUse {
    #[serde(rename = "Active")]
    active: bool,
    #[serde(rename = "Power")]
    power: usize,
    #[serde(rename = "ScheduleType")]
    schedule_type: ScheduleType,
    #[serde(rename = "TimeTable")]
    time_table: TimeTable,
    #[serde(rename = "Weekdays")]
    weekdays: Weekdays,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TimesOfUse {
    #[serde(rename = "timeofuse")]
    time_of_use: Vec<TimeOfUse>,
}

/*
 * {
 *	 "errors" : [],
 *	 "permissionFailure" : [],
 *	 "unknownNodes" : [],
 *	 "validationErrors" : [],
 *	 "writeFailure" : [],
 *	 "writeSuccess" :
 *	 [
 *	 	"timeofuse"
 *	 ]
 * }
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TimesOfUseResponse {
    errors: Vec<String>,
    #[serde(rename = "permissionFailure")]
    permission_failure: Vec<String>,
    #[serde(rename = "unknownNodes")]
    unknown_nodes: Vec<String>,
    #[serde(rename = "validationErrors")]
    validation_errors: Vec<String>,
    #[serde(rename = "writeFailure")]
    write_failure: Vec<String>,
    #[serde(rename = "writeSuccess")]
    write_success: Vec<String>,
}

//-------------------------------------------------------------------------------------------------

pub struct FroniusApi {
    digest_auth: DigestAuth,
    fronius_config: Fronius,
}

//-------------------------------------------------------------------------------------------------

impl FroniusApi {
    pub fn new(fronius_config: &Fronius) -> Self {
        Self {
            digest_auth: DigestAuth::new(&fronius_config.username, &fronius_config.password),
            fronius_config: fronius_config.clone(),
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

    pub fn block_battery_for_duration(
        &mut self,
        duration_to_block: &Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Early login because if login does not work nothing else SHALL happen!
        self.login()?;

        let time_table_format = "%H:%M";

        let start = Local::now();
        let end = start + *duration_to_block;

        let formatted_start = start.format(time_table_format);
        let formatted_end = end.format(time_table_format);

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

        Ok(())
    }

    pub fn fully_unblock_battery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }
}
