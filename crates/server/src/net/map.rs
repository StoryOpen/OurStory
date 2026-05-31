use crate::config::Config;

pub async fn run(_config: Config) {
    tracing::info!("standalone map server starting");
    // TODO: accept player TCP connections for dedicated boss map instances
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
