use tokio::sync::{Mutex, mpsc};
use std::sync::Arc;

use number_server::{start_listening, start_logger, start_reporter, AppStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let status = Arc::new(Mutex::new(AppStatus::new()));
    let (tx, rx) = mpsc::unbounded_channel();

    let listener = start_listening(status.clone(), tx.clone(), 4000);
    let logger = start_logger(rx);
    let reporter = start_reporter(status.clone());

    match tokio::try_join!(
        listener,
        logger,
        reporter
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }

}

