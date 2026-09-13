mod rip_deamon;
use crate::http::{create_http_channel, spawn_http_server};
use rip_deamon::RipDeamon;
use routing_table::RoutingTable;
use routing_table_rtnetlink::RtNetlinkRoutingTableDriver;
mod address;
mod cfg;
mod http;
mod logger;
mod result;
mod rip_packet;
mod rip_socket;
mod rip_updater;
mod routing_table;
mod routing_table_rtnetlink;
#[cfg(test)]
mod routing_table_stub;
use result::RipResult;
use std::{env, path::PathBuf};
mod common;
mod rip_database;
mod rip_ifc;
mod rip_route;

struct ProgramArgs {
    cfg_path: String,
    log_file_path: Option<PathBuf>,
}

fn get_program_args() -> RipResult<ProgramArgs> {
    let mut args_iter = env::args().into_iter();
    args_iter.next().expect("program name");
    let cfg_path = args_iter.next().ok_or(result::RipError::InvalidArgument(
        "missing configuration path, usage: rip_v2 <config.yml> [log_file]".to_string(),
    ))?;
    let log_file_path = args_iter.next().map(PathBuf::from);

    if args_iter.next().is_some() {
        return Err(result::RipError::InvalidArgument(
            "too many arguments, usage: rip_v2 <config.yml> [log_file]".to_string(),
        ));
    }

    Ok(ProgramArgs {
        cfg_path,
        log_file_path,
    })
}

async fn run_rip_deamon(args: ProgramArgs) -> RipResult<()> {
    let routing_driver = RtNetlinkRoutingTableDriver::new().await?;
    let routing_table = RoutingTable::with_driver(routing_driver);
    let (http_request_tx, http_request_rx) = create_http_channel();
    let mut deamon = RipDeamon::new(routing_table, http_request_rx);

    log::info!("starting RIP daemon with configuration {}", args.cfg_path);
    deamon.setup(args.cfg_path.as_str())?;
    spawn_http_server(http_request_tx, args.log_file_path).await?;
    deamon.run().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = match get_program_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err.to_string());
            std::process::exit(1);
        }
    };
    let logger_config = match logger::init_logger(args.log_file_path.clone()) {
        Ok(logger_config) => logger_config,
        Err(err) => {
            eprintln!("{}", err.to_string());
            std::process::exit(1);
        }
    };
    let logger::LoggerConfig {
        handle: logger_handle,
        log_file_path,
    } = logger_config;

    if let Err(err) = run_rip_deamon(ProgramArgs {
        cfg_path: args.cfg_path,
        log_file_path,
    })
    .await
    {
        log::error!("{}", err.to_string());
        std::process::exit(1);
    }

    drop(logger_handle);
}
