pub trait Observation: Clone + Send + Sync {}

impl<T> Observation for T where T: Clone + Send + Sync {}
