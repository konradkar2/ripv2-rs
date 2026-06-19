use crate::cfg::RipConfiguration;
use crate::common;
use crate::common::Error;
use crate::common::Result;
use crate::rip_socket::SocketPair;
use crate::rip_updater::RipUpdater;
use crate::routing_table::RoutingTable;
use std::future;
use std::io;
use std::pin::Pin;
use tokio::time::{self, Duration, Instant, Sleep};
pub struct RipDeamon {
    routing_table: RoutingTable,
    sockets: Vec<SocketPair>,
    //updater: RipUpdater,
}

async fn wait_optional_timer(timer: &mut Option<Pin<Box<Sleep>>>) {
    match timer {
        Some(timer) => timer.as_mut().await,
        None => future::pending::<()>().await,
    }
}

impl RipDeamon {
    pub fn new() -> Self {
        return Self {
            routing_table: RoutingTable::new(),
            sockets: vec![],
            //updater: RipUpdater {  }
        };
    }

    fn setup_sockets(&mut self, cfg: &RipConfiguration) -> Result<()> {
        for ifc_cfg in cfg.rip_interfaces.iter() {
            let if_name = ifc_cfg.dev.clone();

            let socket_pair =
                SocketPair::create_and_configure(if_name.as_str()).map_err(|err| {
                    return Error::FailedToConfigureInterface {
                        msg: err.to_string(),
                        if_name,
                    };
                })?;
            self.sockets.push(socket_pair);
        }

        Ok(())
    }

    pub fn setup(&mut self, cfg_path: &str) -> Result<()> {
        let rip_cfg = RipConfiguration::read_and_parse(cfg_path)?;
        self.setup_sockets(&rip_cfg)?;

        Ok(())
    }

    // fn handle_rx_data(&self, ) {
    //     for socket_pair in self.sockets.iter() {
    //         let rx_socket = &socket_pair.rx;
    //         rx_socket.socket.poll_recv_from(cx, buf)

    //     }
    // }

    pub async fn run(&self) -> common::Result<()> {
    let mut warmup_timer = Some(Box::pin(time::sleep(Duration::from_secs(3))));
    
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);


    loop {
        tokio::select! {
            _ = wait_optional_timer(&mut warmup_timer) => {
                warmup_timer = None;

                println!("Warmup timer triggered");
                RipUpdater::rip_send_request_multicast(&self.sockets).await
                    .map_err(|err| common::Error::IoError(err.to_string()))?;
            }

            _ = poll_sockets(&self) => {

            }

            // tutaj odbiór z socketu:
            // result = self.recv_rip_message() => {
            //     ...
            // }
        }
    }
}
}
