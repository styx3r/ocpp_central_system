use crate::MessageTypeName;
use config::config::SmartChargingMode;
use rust_ocpp::v1_6::types::{ChargePointStatus, ChargingProfile};

//-------------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub id_tag: Option<String>,
    pub transaction_id: i32,
    pub meter_value_start: i32,
    pub meter_value_stop: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RequestToSend {
    pub uuid: String,
    pub message_type: MessageTypeName,
    pub payload: String,
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct Measurand {
    /// Instantaneous current flow to EV in A. Sum of all phases.
    pub(crate) current_import: Option<f64>,
    /// Maximum current offered to EV in A.
    pub(crate) current_offered: Option<f64>,
    /// Active electrical energy imported from the grid supply in Wh.
    pub(crate) energy_active_import_register: Option<f64>,
    /// Reactive electrical energy imported from the grid supply in Varh.
    pub(crate) energy_reactive_import_register: Option<f64>,
    /// Frequency in Hz.
    pub(crate) frequency: Option<f64>,
    /// Instantaneous active power imported by EV in kW. Sum of all phases.
    pub(crate) power_active_import: Option<f64>,
    /// Instantaneous RMS for AC supply voltage in V. Sum of all phases.
    pub(crate) voltage: Option<f64>,
    /// Maximum power offered to EV in kW.
    pub(crate) power_offered: Option<f64>,
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct ChargePointState {
    /// ChargePointStatus as defined in the OCPP v1.6 specification.
    charge_point_status: Option<ChargePointStatus>,

    /// Measurand values received by a MeterValuesRequest.
    pub(crate) measurand: Measurand,

    /// Calculated cos(phi). Will be populated on the first received MeterValuesRequest.
    pub(crate) latest_cos_phi: Option<f64>,

    /// Max. charging current calculated with I = P / (U * cos(phi)). Where P is the maximum power
    /// specified in the config which shall be offered.
    pub(crate) max_current: Option<f64>,

    /// Pending requests to send to the ChargingPoint.
    pub(crate) requests_to_send: Vec<RequestToSend>,

    /// Requests which are awaiting responses from the ChargingPoint.
    pub(crate) requests_awaiting_confirmation: Vec<RequestToSend>,

    /// Currently running transactions. AFAIK only one Transaction is possible since the whole
    /// library is single threaded.
    pub(crate) running_transactions: Vec<Transaction>,

    /// Indicates which SmartChargingMode is currently used.
    smart_charging_mode: SmartChargingMode,

    /// Currently active ChargingProfiles
    active_charging_profiles: Vec<ChargingProfile>,
}

impl ChargePointState {
    pub fn new(cos_phi: f64, power: f64, current: f64, voltage: f64) -> Self {
        Self {
            charge_point_status: None,
            latest_cos_phi: Some(cos_phi),
            measurand: Measurand {
                current_import: None,
                current_offered: Some(current),
                energy_active_import_register: None,
                energy_reactive_import_register: None,
                frequency: None,
                power_active_import: None,
                power_offered: Some(power),
                voltage: Some(voltage),
            },
            max_current: None,
            requests_to_send: vec![],
            requests_awaiting_confirmation: vec![],
            running_transactions: vec![],
            smart_charging_mode: SmartChargingMode::Instant,
            active_charging_profiles: vec![],
        }
    }

    pub fn get_charge_point_status(&self) -> &Option<ChargePointStatus> {
        &self.charge_point_status
    }

    pub fn get_latest_cos_phi(&self) -> Option<f64> {
        self.latest_cos_phi
    }

    pub fn get_latest_power_offered(&self) -> Option<f64> {
        self.measurand.power_offered
    }

    pub fn get_latest_current_offered(&self) -> Option<f64> {
        self.measurand.current_offered
    }

    pub fn get_latest_voltage(&self) -> Option<f64> {
        self.measurand.voltage
    }

    pub fn get_latest_power_active_imported(&self) -> Option<f64> {
        self.measurand.power_active_import
    }

    pub fn get_max_current(&self) -> Option<f64> {
        self.max_current
    }

    pub fn get_requests_to_send(&self) -> &Vec<RequestToSend> {
        &self.requests_to_send
    }

    pub fn get_running_transaction_ids(&self) -> &Vec<Transaction> {
        &self.running_transactions
    }

    pub fn get_smart_charging_mode(&self) -> SmartChargingMode {
        self.smart_charging_mode
    }

    pub fn get_active_charging_profiles(&self) -> &Vec<ChargingProfile> {
        &self.active_charging_profiles
    }

    pub fn get_active_charging_profile(
        &self,
        charging_profile_id: i32,
    ) -> Option<&ChargingProfile> {
        self.active_charging_profiles
            .iter()
            .find(|&charging_profile| charging_profile.charging_profile_id == charging_profile_id)
    }

    pub fn add_running_transaction_id(&mut self, transaction: Transaction) {
        self.running_transactions.push(transaction);
    }

    pub fn add_charging_profile(&mut self, charging_profile: &ChargingProfile) {
        self.active_charging_profiles.push(charging_profile.clone());
    }

    pub fn remove_charging_profile(&mut self, charging_profile_id: i32) {
        self.active_charging_profiles
            .retain(|charging_profile| charging_profile.charging_profile_id != charging_profile_id);
    }

    pub fn set_charge_point_status(&mut self, status: ChargePointStatus) {
        self.charge_point_status = Some(status);
    }

    pub fn set_latest_cos_phi(&mut self, cos_phi: f64) {
        self.latest_cos_phi = Some(cos_phi);
    }

    pub fn set_max_current(&mut self, max_current: f64) {
        self.max_current = Some(max_current);
    }

    pub fn add_request_to_send(&mut self, request_to_send: RequestToSend) {
        self.requests_to_send.push(request_to_send);
    }

    pub fn set_smart_charging_mode(&mut self, mode: SmartChargingMode) {
        self.smart_charging_mode = mode;
    }

    pub fn disable_smart_charging(&mut self) {
        // Resetting all import measurands
        self.measurand.current_import = None;
        self.measurand.energy_active_import_register = None;
        self.measurand.energy_reactive_import_register = None;
        self.measurand.power_active_import = None;

        self.smart_charging_mode = SmartChargingMode::Instant;
    }
}
