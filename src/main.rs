mod rip_deamon;
use rip_deamon::RipDeamon;
use routing_table::RoutingTable;
use routing_table_rtnetlink::RtNetlinkRoutingTableDriver;
mod address;
mod cfg;
mod result;
mod rip_packet;
mod rip_socket;
mod rip_updater;
mod routing_table;
mod routing_table_rtnetlink;
#[cfg(test)]
mod routing_table_stub;
use result::RipResult;
use std::env;
mod common;
mod rip_database;
mod rip_ifc;
mod rip_route;

fn get_cfg_path() -> RipResult<String> {
    let mut args_iter = env::args().into_iter();
    args_iter.next().expect("program name");
    args_iter.next().ok_or(result::RipError::InvalidArgument(
        "missing configuration path".to_string(),
    ))
}

async fn run_rip_deamon() -> RipResult<()> {
    let routing_driver = RtNetlinkRoutingTableDriver::new().await?;
    let routing_table = RoutingTable::with_driver(routing_driver);
    let mut deamon = RipDeamon::with_routing_table(routing_table);
    let cfg_path = get_cfg_path()?;

    log::info!("starting RIP daemon with configuration {}", cfg_path);
    deamon.setup(cfg_path.as_str())?;
    deamon.run().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(err) = run_rip_deamon().await {
        log::error!("{}", err.to_string());
        std::process::exit(1);
    }
}
