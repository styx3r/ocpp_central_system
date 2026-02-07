use crate::{FroniusApi, api_types::PowerFlowRealtimeData};

#[derive(Default)]
pub struct FroniusMock {
    pub unblock_battery_called: bool,
    pub block_battery_for_duration_called: bool,
    pub power_flow_realtime_data: Option<PowerFlowRealtimeData>,
}

impl FroniusApi for FroniusMock {
    fn fully_unblock_battery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.unblock_battery_called = true;
        Ok(())
    }

    fn block_battery_for_duration(
        &mut self,
        _duration_to_block: &std::time::Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.block_battery_for_duration_called = true;
        Ok(())
    }

    fn get_power_flow_realtime_data(
        &mut self,
    ) -> Result<PowerFlowRealtimeData, Box<dyn std::error::Error>> {
        Ok(self
            .power_flow_realtime_data
            .clone()
            .expect("Forgot to set power_flow_realtime_data"))
    }
}
