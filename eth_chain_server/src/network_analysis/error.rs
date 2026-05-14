use eyre::Report;

#[derive(Debug)]
pub enum StartNetworkAnalysisError {
    InvalidRequest(Report),
    Spawn(std::io::Error),
}
