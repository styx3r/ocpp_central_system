use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ApiVersions {
    #[serde(alias = "ComponentsApi")]
    pub components_api: String,
    #[serde(alias = "commandsApi")]
    pub commands_api: String,
    #[serde(alias = "configApi")]
    pub config_api: String,
    #[serde(alias = "setupAppApi")]
    pub setup_app_api: String,
    #[serde(alias = "setupAppUpdateApi")]
    pub setup_app_update_api: String,
    #[serde(alias = "statusApi")]
    pub status_api: String,
    #[serde(alias = "updateApi")]
    pub update_api: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AppVersions {
    #[serde(alias = "minAndroidVersion")]
    pub min_android_version: usize,
    #[serde(alias = "minIOSVersion")]
    pub min_ios_version: usize,
    #[serde(alias = "minWinVersion")]
    pub min_win_version: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ApiVersionResponse {
    #[serde(alias = "apiversions")]
    pub api_versions: ApiVersions,
    #[serde(alias = "articleNumber")]
    pub article_number: String,
    #[serde(alias = "commonName")]
    pub common_name: String,
    #[serde(alias = "devicegroup")]
    pub device_group: String,
    #[serde(alias = "devicename")]
    pub device_name: String,
    #[serde(alias = "hardwareId")]
    pub hardware_id: String,
    #[serde(alias = "hwrevisions")]
    pub hardware_revisions: HashMap<String, String>,
    #[serde(alias = "minAppVersions")]
    pub min_app_versions: AppVersions,
    #[serde(alias = "numberOfPhases")]
    pub number_of_phases: usize,
    #[serde(alias = "serialNumber")]
    pub serial_number: String,
    #[serde(alias = "softwareVersionPrefix")]
    pub software_version_prefix: String,
    #[serde(alias = "swrevisions")]
    pub software_revisions: HashMap<String, String>,
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
pub struct TimeTable {
    #[serde(rename = "Start")]
    pub start: String,
    #[serde(rename = "End")]
    pub end: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct Weekdays {
    #[serde(rename = "Mon")]
    pub monday: bool,
    #[serde(rename = "Tue")]
    pub tuesday: bool,
    #[serde(rename = "Wed")]
    pub wednesday: bool,
    #[serde(rename = "Thu")]
    pub thursday: bool,
    #[serde(rename = "Fri")]
    pub friday: bool,
    #[serde(rename = "Sat")]
    pub saturday: bool,
    #[serde(rename = "Sun")]
    pub sunday: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum ScheduleType {
    #[serde(rename = "DISCHARGE_MAX")]
    DischargeMax,
    #[serde(rename = "DISCHARGE_MIN")]
    DischargeMin,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TimeOfUse {
    #[serde(rename = "Active")]
    pub active: bool,
    #[serde(rename = "Power")]
    pub power: usize,
    #[serde(rename = "ScheduleType")]
    pub schedule_type: ScheduleType,
    #[serde(rename = "TimeTable")]
    pub time_table: TimeTable,
    #[serde(rename = "Weekdays")]
    pub weekdays: Weekdays,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TimesOfUse {
    #[serde(rename = "timeofuse")]
    pub time_of_use: Vec<TimeOfUse>,
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
pub struct TimesOfUseResponse {
    pub errors: Vec<String>,
    #[serde(rename = "permissionFailure")]
    pub permission_failure: Vec<String>,
    #[serde(rename = "unknownNodes")]
    pub unknown_nodes: Vec<String>,
    #[serde(rename = "validationErrors")]
    pub validation_errors: Vec<String>,
    #[serde(rename = "writeFailure")]
    pub write_failure: Vec<String>,
    #[serde(rename = "writeSuccess")]
    pub write_success: Vec<String>,
}

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ResultData {
    pub roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LoginResponse {
    #[serde(alias = "resultData")]
    pub result_data: ResultData,
    pub success: bool,
}
