use clap::Parser;
use open_delay_core::Config;
use open_delay_service::{default_data_dir, start, Error};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Open-Delay independent local RTMP delay engine")]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    ui_dir: Option<PathBuf>,
    #[arg(long)]
    api_port: Option<u16>,
    #[arg(long)]
    rtmp_port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let data = args.data_dir.unwrap_or_else(default_data_dir);
    let config_path = args.config.unwrap_or_else(|| data.join("config.json"));
    let mut config = Config::load(&config_path)?;
    if let Some(port) = args.api_port {
        config.preferences.api_port = port;
    }
    if let Some(port) = args.rtmp_port {
        config.preferences.rtmp_port = port;
    }
    let ui = args.ui_dir.unwrap_or_else(|| PathBuf::from("ui/dist"));
    let running = start(config, config_path, data, ui).await?;
    println!(
        "Open-Delay API http://{} / RTMP {}",
        running.api_addr, running.rtmp_addr
    );
    tokio::select! {
        result=tokio::signal::ctrl_c()=>result?,
        _=running.service.shutdown.cancelled()=>{},
    }
    running.stop().await;
    Ok(())
}
