use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Status {
    #[serde(alias = "Code")]
    pub code: i64,
    #[serde(alias = "Reason")]
    pub reason: String,
    #[serde(alias = "UserMessage")]
    pub user_message: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PowerFlowRealtimeDataHeader {
    #[serde(alias = "RequestArguments")]
    pub request_arguments: std::collections::HashMap<String, String>,
    #[serde(alias = "Status")]
    pub status: Status,
    #[serde(alias = "Timestamp")]
    pub timestamp: DateTime<Utc>
}

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Inverter {
    #[serde(alias = "Battery_Mode")]
    pub battery_mode: String,
    #[serde(alias = "DT")]
    pub dt: u64,
    #[serde(alias = "E_Day")]
    pub e_day: Option<f64>,
    #[serde(alias = "E_Total")]
    pub e_total: Option<f64>,
    #[serde(alias = "E_Year")]
    pub e_year: Option<f64>,
    #[serde(alias = "P")]
    pub power: Option<f64>,
    #[serde(alias = "SOC")]
    pub soc: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Site {
    #[serde(alias = "Mode")]
    pub mode: String,
    #[serde(alias = "BatteryStandby")]
    pub battery_standby: bool,
    #[serde(alias = "BackupMode")]
    pub backup_mode: bool,
    #[serde(alias = "P_Grid")]
    pub p_grid: Option<f64>,
    #[serde(alias = "P_Load")]
    pub p_load: Option<f64>,
    #[serde(alias = "P_Akku")]
    pub p_akku: Option<f64>,
    #[serde(alias = "P_PV")]
    pub p_pv: Option<f64>,
    #[serde(alias = "rel_SelfConsumption")]
    pub rel_self_consumption: Option<f64>,
    #[serde(alias = "rel_Autonomy")]
    pub rel_autonomy: Option<f64>,
    #[serde(alias = "Meter_Location")]
    pub meter_location: String,
    #[serde(alias = "E_Day")]
    pub e_day: Option<f64>,
    #[serde(alias = "E_Year")]
    pub e_year: Option<f64>,
    #[serde(alias = "E_Total")]
    pub e_total: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Ohmpilot {
    #[serde(alias = "P_AC_Total")]
    pub p_ac_total: f64,
    #[serde(alias = "State")]
    pub state: String,
    #[serde(alias = "Temperature")]
    pub temperature: f64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct OhmpilotEco {
    #[serde(alias = "P_AC_Total")]
    pub p_ac_total: f64,
    #[serde(alias = "State_HR1")]
    pub state_hr1: String,
    #[serde(alias = "State_HR2")]
    pub state_hr2: String,
    #[serde(alias = "Temperature_1")]
    pub temperature_1: f64,
    #[serde(alias = "Temperature_2")]
    pub temperature_2: f64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Smartloads {
    #[serde(alias = "Ohmpilots")]
    pub ohmpilots: std::collections::HashMap<String, Ohmpilot>,
    #[serde(alias = "OhmpilotEcos")]
    pub ohmpilot_ecos: std::collections::HashMap<String, OhmpilotEco>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SecondaryMeters {
    #[serde(alias = "P")]
    pub power: f64,
    #[serde(alias = "MLoc")]
    pub meter_location: f64,
    #[serde(alias = "Label")]
    pub label: String,
    #[serde(alias = "Category")]
    pub category: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Data {
    #[serde(alias = "Inverters")]
    pub inverters: std::collections::HashMap<String, Inverter>,
    #[serde(alias = "Site")]
    pub site: Site,
    #[serde(alias = "Smartloads")]
    pub smartloads: Smartloads,
    #[serde(alias = "SecondaryMeters")]
    pub secondart_meters: std::collections::HashMap<String, SecondaryMeters>,
    #[serde(alias = "Version")]
    pub version: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PowerFlowRealtimeDataBody {
    #[serde(alias = "Data")]
    pub data: Data,
}

//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PowerFlowRealtimeData {
    #[serde(alias = "Body")]
    pub body: PowerFlowRealtimeDataBody,
    #[serde(alias = "Head")]
    pub head: PowerFlowRealtimeDataHeader,
}
