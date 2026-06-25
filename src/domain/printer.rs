#[derive(Debug)]
pub struct PrinterInfo {
    pub name: String,
    pub state: String,
    pub queued_jobs: u32,
}
