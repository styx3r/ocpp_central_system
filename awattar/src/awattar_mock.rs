use crate::AwattarApi;

//-------------------------------------------------------------------------------------------------

pub struct AwattarApiMock {
    period: Option<crate::Period>
}

//-------------------------------------------------------------------------------------------------

impl AwattarApiMock {
    pub fn default() -> Self {
        Self {
            period: None
        }
    }

    pub fn set_response(&mut self, period_response: crate::Period) {
        self.period = Some(period_response);
    }
}

//-------------------------------------------------------------------------------------------------

impl AwattarApi for AwattarApiMock {
    fn update_price_chart(
            &self,
            _config: &config::config::Config,
        ) -> Result<crate::Period, Box<dyn std::error::Error>> {
       Ok(self.period.clone().unwrap())
    }
}
