use std::sync::Arc;

use crate::adapters::cups::CupsClient;
use crate::services::PrinterService;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<PrinterService>,
    pub cups_addr: String,
}

impl AppState {
    pub fn new(cups_host: &str, cups_port: u16) -> Self {
        let client = CupsClient::new(cups_host, cups_port);
        Self {
            service: Arc::new(PrinterService::new(client)),
            cups_addr: format!("{cups_host}:{cups_port}"),
        }
    }
}
