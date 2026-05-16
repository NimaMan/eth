use eyre::Report;

#[derive(Debug)]
pub enum StartTokenNetworkAnalysisError {
    InvalidRequest(Report),
    Spawn(std::io::Error),
}
