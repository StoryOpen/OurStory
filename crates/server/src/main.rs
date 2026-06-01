mod config;
mod events;
mod handlers;
mod net;

use clap::Parser;
use config::Config;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "server", about = "OurStory game server")]
struct Args {
    #[arg(long, default_value = "config.yaml")]
    config: String,

    #[arg(long, default_value = "channel")]
    role: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config).expect("failed to load config");

    match args.role.as_str() {
        "login" => net::login::run(config).await,
        "world" => net::world::run(config).await,
        "channel" => net::channel::run(config).await,
        "map" => net::map::run(config).await,
        other => panic!("unknown role: {other}"),
    }
}
