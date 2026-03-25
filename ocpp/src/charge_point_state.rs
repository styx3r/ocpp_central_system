use crate::MessageTypeName;
use config::config::SmartChargingMode;
use rust_ocpp::v1_6::types::{ChargePointStatus, ChargingProfile};
use serde::{Deserialize, Serialize};
use std::fmt;

use uom::si::f64::*;

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    L1,
    L2,
    L3,
    N,
    L1N,
    L2N,
    L3N,
    L1L2,
    L2L3,
    L3L1,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Phase::L1 => write!(f, "L1"),
            Phase::L2 => write!(f, "L2"),
            Phase::L3 => write!(f, "L3"),
            Phase::N => write!(f, "N"),
            Phase::L1N => write!(f, "L1N"),
            Phase::L2N => write!(f, "L2N"),
            Phase::L3N => write!(f, "L3N"),
            Phase::L1L2 => write!(f, "L1L2"),
            Phase::L2L3 => write!(f, "L2L3"),
            Phase::L3L1 => write!(f, "L3L1"),
        }
    }
}

impl From<rust_ocpp::v1_6::types::Phase> for Phase {
    fn from(value: rust_ocpp::v1_6::types::Phase) -> Self {
        match value {
            rust_ocpp::v1_6::types::Phase::L1 => Self::L1,
            rust_ocpp::v1_6::types::Phase::L2 => Self::L2,
            rust_ocpp::v1_6::types::Phase::L3 => Self::L3,
            rust_ocpp::v1_6::types::Phase::N => Self::N,
            rust_ocpp::v1_6::types::Phase::L1N => Self::L1N,
            rust_ocpp::v1_6::types::Phase::L2N => Self::L2N,
            rust_ocpp::v1_6::types::Phase::L3N => Self::L3N,
            rust_ocpp::v1_6::types::Phase::L1L2 => Self::L1L2,
            rust_ocpp::v1_6::types::Phase::L2L3 => Self::L2L3,
            rust_ocpp::v1_6::types::Phase::L3L1 => Self::L3L1,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PhaseMeasurand<T> {
    pub value: T,
    pub phase: Phase,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MultiPhaseMeasurand<T> {
    pub(crate) measurands: Vec<PhaseMeasurand<T>>,
}

impl<T> Default for MultiPhaseMeasurand<T> {
    fn default() -> Self {
        MultiPhaseMeasurand { measurands: vec![] }
    }
}

impl<T: std::iter::Sum + Copy> MultiPhaseMeasurand<T> {
    pub fn get_sum_of_phases(&self, phases: &[Phase]) -> Option<T> {
        let filtered_phases = self
            .measurands
            .iter()
            .filter(|m| phases.contains(&m.phase))
            .map(|m| m.value)
            .collect::<Vec<T>>();

        if filtered_phases.is_empty() {
            return None;
        }

        Some(filtered_phases.into_iter().sum::<T>())
    }

    pub fn get_phase(&self, phase: Phase) -> Option<PhaseMeasurand<T>> {
        self.measurands
            .iter()
            .find(|m| m.phase == phase)
            .map(|m| m.to_owned())
    }
}

impl<T> MultiPhaseMeasurand<T> {
    pub fn new(measurands: Vec<PhaseMeasurand<T>>) -> Self {
        Self { measurands }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct Measurand {
    /// Instantaneous current flow from EV in A.
    pub current_export: MultiPhaseMeasurand<ElectricCurrent>,
    /// Instantaneous current flow to EV in A.
    pub current_import: MultiPhaseMeasurand<ElectricCurrent>,
    /// Maximum current offered to EV in A.
    pub current_offered: Option<ElectricCurrent>,
    /// Active electrical energy exported to the grid in Wh.
    pub energy_active_export_register: Option<Energy>,
    /// Active electrical energy imported from the grid supply in Wh.
    pub energy_active_import_register: Option<Energy>,
    /// Reactive electrical energy exported to the grid in Varh.
    pub energy_reactive_export_register: Option<Energy>,
    /// Reactive electrical energy imported from the grid supply in Varh.
    pub energy_reactive_import_register: Option<Energy>,
    /// Absolute amount of electrical energy Wh exported to the grid within a given interval.
    pub energy_active_export_interval: Option<Energy>,
    /// Absolute amount of electrical energy Wh imported from the grid within a given interval.
    pub energy_active_import_interval: Option<Energy>,
    /// Absolute amount of reactive electrical energy Varh exported to the grid within a given interval.
    pub energy_reactive_export_interval: Option<Energy>,
    /// Absolute amount of reactive electrical energy Varh imported from the grid within a given interval.
    pub energy_reactive_import_interval: Option<Energy>,
    /// Frequency in Hz.
    pub frequency: Option<Frequency>,
    /// Instantaneous active power exported by EV in W.
    pub power_active_export: MultiPhaseMeasurand<Power>,
    /// Instantaneous active power imported by EV in W.
    pub power_active_import: MultiPhaseMeasurand<Power>,
    /// Instantaneous power factor of total energy flow.
    pub power_factor: Option<f64>,
    /// Maximum power offered to EV in W.
    pub power_offered: Option<Power>,
    /// Instantaneous reactive power exported by EV in Var = Wr.
    pub power_reactive_export: MultiPhaseMeasurand<Power>,
    /// Instantaneous reactive power imported by EV in Var = Wr.
    pub power_reactive_import: MultiPhaseMeasurand<Power>,
    /// Fan speed in RPM.
    pub rpm: Option<f64>,
    /// State of charge of charging vehicle in percentage.
    pub state_of_charge: Option<f64>,
    /// Temperature reading inside ChargePoint.
    pub temperature: Option<TemperatureInterval>,
    /// Instantaneous RMS for AC supply voltage in V.
    pub voltage: MultiPhaseMeasurand<ElectricPotential>,
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct ChargePointState {
    /// ChargePointStatus as defined in the OCPP v1.6 specification.
    charge_point_status: Option<ChargePointStatus>,

    /// Measurand values received by a MeterValuesRequest.
    pub(crate) measurand: Measurand,

    /// Max. charging current calculated with I = P / (U * cos(phi)). Where P is the maximum power
    /// specified in the config which shall be offered.
    pub(crate) max_current: Option<ElectricCurrent>,

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
    pub fn new(
        power: Power,
        current: ElectricCurrent,
        voltage: MultiPhaseMeasurand<ElectricPotential>,
    ) -> Self {
        Self {
            charge_point_status: None,
            measurand: Measurand {
                current_export: MultiPhaseMeasurand::default(),
                current_import: MultiPhaseMeasurand::default(),
                current_offered: Some(current),
                energy_active_export_register: None,
                energy_active_import_register: None,
                energy_reactive_export_register: None,
                energy_reactive_import_register: None,
                energy_active_export_interval: None,
                energy_active_import_interval: None,
                energy_reactive_export_interval: None,
                energy_reactive_import_interval: None,
                frequency: None,
                power_active_export: MultiPhaseMeasurand::default(),
                power_active_import: MultiPhaseMeasurand::default(),
                power_factor: None,
                power_offered: Some(power),
                power_reactive_export: MultiPhaseMeasurand::default(),
                power_reactive_import: MultiPhaseMeasurand::default(),
                rpm: None,
                state_of_charge: None,
                temperature: None,
                voltage,
            },
            max_current: None,
            requests_to_send: vec![],
            requests_awaiting_confirmation: vec![],
            running_transactions: vec![],
            smart_charging_mode: SmartChargingMode::Instant,
            active_charging_profiles: vec![],
        }
    }

    pub fn with_initial_requests(requests_to_send: Vec<RequestToSend>) -> Self {
        let mut instance = Self::default();
        instance.requests_to_send = requests_to_send;

        instance
    }

    pub fn get_charge_point_status(&self) -> &Option<ChargePointStatus> {
        &self.charge_point_status
    }

    pub fn get_latest_power_offered(&self) -> Option<Power> {
        self.measurand.power_offered
    }

    pub fn get_latest_current_offered(&self) -> Option<ElectricCurrent> {
        self.measurand.current_offered
    }

    pub fn get_latest_voltage(&self) -> MultiPhaseMeasurand<ElectricPotential> {
        self.measurand.voltage.clone()
    }

    pub fn get_latest_power_active_imported(&self) -> MultiPhaseMeasurand<Power> {
        self.measurand.power_active_import.clone()
    }

    pub fn get_max_current(&self) -> Option<ElectricCurrent> {
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

    pub fn get_measurand(&self) -> &Measurand {
        &self.measurand
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

    pub fn set_max_current(&mut self, max_current: ElectricCurrent) {
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
        self.measurand.current_import = MultiPhaseMeasurand::default();
        self.measurand.energy_active_import_register = None;
        self.measurand.energy_reactive_import_register = None;
        self.measurand.power_active_import = MultiPhaseMeasurand::default();

        self.smart_charging_mode = SmartChargingMode::Instant;
    }
}
