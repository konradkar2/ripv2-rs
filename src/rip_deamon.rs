use crate::cfg::parse;
use crate::rip_socket::RipSocket;
use crate::routing_table::RoutingTable;
use std::env;
use std::fs;
use tokio::time::{self, Duration, Instant};
use crate::common::Result;

pub struct RipDeamon {
    routing_table: RoutingTable,
    sockets: Vec<RipSocket>,
}

impl RipDeamon {
    pub fn new() -> Self {
        return Self {
            routing_table: RoutingTable::new(),
            sockets: vec![],
        };
    }

    pub fn setup(&self, cfg_path: &str) -> Result<()>{
        let contents =
            fs::read_to_string(cfg_path).expect("Should have been able to read the file");
        let _rip_cfg = parse(contents.as_str())?;

        Ok(())
    }

    pub async fn run(self: &Self) {
        let route_timeout_1 = time::sleep(Duration::from_secs(5));
        tokio::pin!(route_timeout_1);
        loop {
            tokio::select! {
                 _ = &mut route_timeout_1 => {
                     println!("RipDeamon timer triggered");

                    route_timeout_1
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(5));
                }
            }
        }
    }
}
