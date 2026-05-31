use crate::config::Config;

pub async fn run(_config: Config) {
    tracing::info!("world server starting");
    // TODO: listen for channel connections, coordinate parties/guilds
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
