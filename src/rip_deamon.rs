use crate::cfg::RipConfiguration;
use crate::common;
use crate::common::Error;
use crate::common::Result;
use crate::rip_socket::SocketPair;
use crate::rip_updater::RipUpdater;
use crate::routing_table::RoutingTable;
use std::io;
use tokio::time::{self, Duration, Instant};
pub struct RipDeamon {
    routing_table: RoutingTable,
    sockets: Vec<SocketPair>,
    //updater: RipUpdater,
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

    pub async fn run(self: &Self) -> common::Result<()> {
        let route_timeout_1 = time::sleep(Duration::from_secs(5));
        tokio::pin!(route_timeout_1);
        loop {
            tokio::select! {
                 _ = &mut route_timeout_1 => {
                    println!("Warmup timer triggered");
                    RipUpdater::rip_send_request_multicast(&self.sockets).map_err(|err| return common::Error::IoError(err.to_string()))?;
                }
            }
        }
    }
}
