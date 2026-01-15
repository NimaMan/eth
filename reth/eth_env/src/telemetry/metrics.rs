#[derive(Default)]
pub struct EnvMetrics {
    pub steps: u64,
    pub predictions_submitted: u64,
}

impl EnvMetrics {
    pub fn record_step(&mut self, submitted_prediction: bool) {
        self.steps += 1;
        if submitted_prediction {
            self.predictions_submitted += 1;
        }
    }
}
