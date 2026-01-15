#[derive(Default)]
pub struct ObservationCache<O> {
    last: Option<O>,
}

impl<O> ObservationCache<O> {
    pub fn update(&mut self, observation: O) {
        self.last = Some(observation);
    }

    pub fn last(&self) -> Option<&O> {
        self.last.as_ref()
    }
}
