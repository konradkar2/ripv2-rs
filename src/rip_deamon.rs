use crate::cfg::RipConfiguration;
use crate::common::Error;
use crate::common::Result;
use crate::rip_socket::RipSocket;
use crate::routing_table::RoutingTable;
use std::io;
use tokio::time::{self, Duration, Instant};

struct SocketPair {
    tx: RipSocket,
    rx: RipSocket,
}

impl SocketPair {
    fn create_and_configure(dev: &str) -> io::Result<SocketPair> {
        let rx = RipSocket::create(dev)?;
        rx.configure_as_multicast_rx()?;

        let tx = RipSocket::create(dev)?;
        tx.configure_as_multicast_tx()?;

        Ok(Self { tx, rx })
    }
}

pub struct RipDeamon {
    routing_table: RoutingTable,
    sockets: Vec<SocketPair>,
}

impl RipDeamon {
    pub fn new() -> Self {
        return Self {
            routing_table: RoutingTable::new(),
            sockets: vec![],
        };
    }

    fn setup_sockets(&mut self, cfg: &RipConfiguration) -> Result<()> {
        for ifc_cfg in cfg.rip_interfaces.iter() {
            let dev = ifc_cfg.dev.as_str();
            
            let socket_pair = SocketPair::create_and_configure(dev).map_err(|err| {
                return Error::OsError(format!("{}: device: {}", err.to_string(), dev));
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
