use crate::prelude::*;
use system_utils::SystemMonitor;

static SYSTEM_MONITOR: State<SystemMonitor> = State::default();

/// API: Returns static system information
#[log(skip_all)]
pub async fn handle_system_info(tx: Sender<Bytes>) -> Result<()> {
    let info = SYSTEM_MONITOR.lock().await.info();
    let msg = format!("{info}");

    tx.send(Chunk::answer(msg)).await
}

/// API: Returns current live system metrics
#[log(skip_all)]
pub async fn handle_system_metrics(tx: Sender<Bytes>) -> Result<()> {
    let metrics = SYSTEM_MONITOR
        .lock()
        .await
        .refresh_metrics_with_interval(Duration::from_secs(10));
    let msg = format!("{metrics}",);

    info!("System metrics collected.");
    tx.send(Chunk::answer(msg)).await
}

/// API: Returns the list of connected devices
#[log(skip_all)]
pub async fn handle_devices_list(tx: Sender<Bytes>) -> Result<()> {
    let devices = SYSTEM_MONITOR
        .lock()
        .await
        .refresh_devices_with_interval(Duration::from_secs(60));
    let msg = format!("{devices}",);

    info!("Connected devices enumerated.");
    tx.send(Chunk::answer(msg)).await
}
