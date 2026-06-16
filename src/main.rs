use tokio::time::{self, Duration, Instant};
mod rip_deamon;
use rip_deamon::RipDeamon;
mod cfg;
mod common;
mod rip_socket;
mod routing_table;
mod rip_updater;
mod ifc;
use common::Result;
use std::env;

fn get_cfg_path() -> Result<String> {
    let mut args_iter = env::args().into_iter();
    args_iter.next().expect("program name");
    args_iter.next().ok_or(common::Error::InvalidArgument(
        "missing configuration path".to_string(),
    ))
}

async fn run_rip_deamon() -> Result<()> {
    let mut deamon = RipDeamon::new();
    let cfg_path = get_cfg_path()?;
    deamon.setup(cfg_path.as_str())?;
    deamon.run().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        let result = run_rip_deamon().await;

        result.map_err(|err| {
            println!("error: {}", err.to_string());
            std::process::exit(1);
        })
    });

    let route_timeout_1 = time::sleep(Duration::from_secs(5));
    tokio::pin!(route_timeout_1);

    let route_timeout_2 = time::sleep(Duration::from_secs(20));
    tokio::pin!(route_timeout_2);

    let route_timeout_3 = time::sleep(Duration::from_secs(60));
    tokio::pin!(route_timeout_3);

    loop {
        tokio::select! {
            _ = &mut route_timeout_1 => {
                println!("timeout_1 expire");

                route_timeout_1
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(5));
            }
            _ = &mut route_timeout_2 => {
                println!("timeout_2 expire");

                route_timeout_2
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(20));
            }
            _ = &mut route_timeout_3 => {
                println!("timeout_3 expire");

                route_timeout_3
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(60));
            }
        }
    }
}
