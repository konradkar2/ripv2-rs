use crate::cfg::{AdvertisedNetwork, RipConfiguration};
use crate::result;
use crate::result::{RipError, RipResult};
use crate::rip_database::RipDatabase;
use crate::rip_ifc::RipIfc;
use crate::rip_packet::{RipIfInfo, RipPacket, RipPacketData};
use crate::rip_route::advertised_network_to_local_route;
use crate::rip_socket::RipSocket;
use crate::rip_updater::RipUpdater;
use crate::routing_table::RoutingTable;
use std::future;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Sleep};
pub struct RipDeamon {
    routing_table: RoutingTable,
    interfaces: Vec<RipIfc>,
    database: RipDatabase,
    updater: RipUpdater,
}

async fn wait_optional_timer(timer: &mut Option<Pin<Box<Sleep>>>) {
    match timer {
        Some(timer) => timer.as_mut().await,
        None => future::pending::<()>().await,
    }
}

fn parse_rx_data(data: &[u8], source: SocketAddr, if_info: RipIfInfo) -> RipResult<RipPacket> {
    let SocketAddr::V4(source) = source else {
        return Err(RipError::InvalidSourceAddress());
    };

    let packet_data = RipPacketData::from_slice(&data)?;

    Ok(RipPacket {
        if_info,
        source,
        data: packet_data,
    })
}

fn spawn_rx_task(rx_socket: RipSocket, sender: mpsc::Sender<RipPacket>) {
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 2048];
        loop {
            let if_name = &rx_socket.if_name;
            let if_info = RipIfInfo {
                if_name: rx_socket.if_name.clone(),
                if_index: rx_socket.if_index,
            };
            match rx_socket.socket.recv_from(buffer.as_mut_slice()).await {
                Ok((len, source_addr)) => {
                    let data = &buffer[..len];
                    let packet = match parse_rx_data(data, source_addr, if_info) {
                        Ok(packet_data) => packet_data,
                        Err(err) => {
                            eprintln!("RIP receive error on {}: {}", if_name, err.to_string());
                            break;
                        }
                    };

                    let _ = sender.send(packet).await;
                }
                Err(error) => {
                    eprintln!("RIP receive error on {}: {}", if_name, error);
                    break;
                }
            }
        }
    });
}

impl RipDeamon {
    pub fn new() -> Self {
        return Self {
            routing_table: RoutingTable::new(),
            interfaces: vec![],
            database: RipDatabase::new(),
            updater: RipUpdater::new(),
        };
    }

    fn setup_sockets(&mut self, cfg: &RipConfiguration) -> RipResult<()> {
        for ifc_cfg in cfg.rip_interfaces.iter() {
            let if_name = ifc_cfg.dev.as_str();

            let rip_inteface = RipIfc::create(if_name)?;
            self.interfaces.push(rip_inteface);
        }

        Ok(())
    }

    fn setup_advertised_network(&mut self, network: &AdvertisedNetwork) -> RipResult<()> {
        let local_route = advertised_network_to_local_route(network)?;
        self.database
            .add_local_route(local_route.entry, local_route.if_index)
    }

    pub fn setup(&mut self, cfg_path: &str) -> RipResult<()> {
        let rip_cfg = RipConfiguration::read_and_parse(cfg_path)?;
        self.setup_sockets(&rip_cfg)?;

        for advertised_network in rip_cfg.advertised_networks {
            self.setup_advertised_network(&advertised_network)?;
        }

        Ok(())
    }

    fn spawn_rx_tasks(&mut self, sender: mpsc::Sender<RipPacket>) {
        for ifc in self.interfaces.iter_mut() {
            if let Some(rx_socket) = ifc.rx.take() {
                spawn_rx_task(rx_socket, sender.clone());
            }
        }
    }

    pub async fn handle_packet(&mut self, packet: RipPacket) -> result::RipResult<()> {
        if packet.is_request() {
            self.updater
                .rip_send_response_unicast(&self.database, &packet.source, &packet.if_info)
                .await
                .map_err(|err| result::RipError::IoError(err.to_string()))?;
        }

        Ok(())
    }

    pub async fn run(&mut self) -> result::RipResult<()> {
        let mut warmup_timer = Some(Box::pin(time::sleep(Duration::from_secs(3))));

        let (tx, mut rx) = tokio::sync::mpsc::channel::<RipPacket>(64);
        self.spawn_rx_tasks(tx);

        loop {
            tokio::select! {
                _ = wait_optional_timer(&mut warmup_timer) => {
                    warmup_timer = None;

                    println!("Warmup timer triggered");
                    for ifc in &self.interfaces {
                        RipUpdater::rip_send_request_multicast(&ifc.tx).await
                        .map_err(|err| result::RipError::IoError(err.to_string()))?;
                    }

                }

                received = rx.recv() => {
                    let Some(packet) = received else {
                        return Err(result::RipError::IoError(
                            "all RIP receive tasks stopped".to_string()
                        ));
                    };

                    println!(
                        "Received packet from {} on {}, with entries count {}",
                        packet.source,
                        packet.if_info.if_name,
                        packet.data.entries.len()
                    );

                    self.handle_packet(packet).await?;
                }

                // tutaj odbiór z socketu:
                // result = self.recv_rip_message() => {
                //     ...
                // }
            }
        }
    }
}
