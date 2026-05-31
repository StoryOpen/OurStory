use crate::config::Config;

pub async fn run(config: Config) {
    tracing::info!("login server starting on port {}", config.login_port);
    // TODO: bind TCP acceptor, handle login packets
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
