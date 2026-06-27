use crate::cfg::{RipConfiguration};
use crate::common;
use crate::common::{RipError, RipResult};
use crate::ifc::{RipPacket, RipPacketData};
use crate::rip_ifc::RipIfc;
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
}

async fn wait_optional_timer(timer: &mut Option<Pin<Box<Sleep>>>) {
    match timer {
        Some(timer) => timer.as_mut().await,
        None => future::pending::<()>().await,
    }
}

fn parse_rx_data(data: &[u8], source_addr: SocketAddr, if_name: &str) -> RipResult<RipPacket> {
    let SocketAddr::V4(source_addr) = source_addr else {
        return Err(RipError::InvalidSourceAddress());
    };

    let packet_data = RipPacketData::from_slice(&data)?;

    Ok(RipPacket {
        if_name: if_name.to_string(),
        source_addr: source_addr.ip().clone(),
        data: packet_data,
    })
}

fn spawn_rx_task(rx_socket: RipSocket, sender: mpsc::Sender<RipPacket>) {
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 2048];
        loop {
            let if_name = &rx_socket.if_name;
            match rx_socket.socket.recv_from(buffer.as_mut_slice()).await {
                Ok((len, source_addr)) => {
                    let data = &buffer[..len];
                    let packet = match parse_rx_data(data, source_addr, if_name) {
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
            //updater: RipUpdater {  }
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

    pub fn setup(&mut self, cfg_path: &str) -> RipResult<()> {
        let rip_cfg = RipConfiguration::read_and_parse(cfg_path)?;
        self.setup_sockets(&rip_cfg)?;

        Ok(())
    }

    fn spawn_rx_tasks(&mut self, sender: mpsc::Sender<RipPacket>) {
        for ifc in self.interfaces.iter_mut() {
            if let Some(rx_socket) = ifc.rx.take() {
                spawn_rx_task(rx_socket, sender.clone());
            }
        }
    }

    pub async fn run(&mut self) -> common::RipResult<()> {
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
                        .map_err(|err| common::RipError::IoError(err.to_string()))?;
                    }

                }

                received = rx.recv() => {
                    let Some(packet) = received else {
                        return Err(common::RipError::IoError(
                            "all RIP receive tasks stopped".to_string()
                        ));
                    };

                    println!(
                        "Received packet from {} on {}, with entries count {}",
                        packet.data.entries.len(),
                        packet.source_addr,
                        packet.if_name
                    );

                    //self.handle_rip_packet(packet)?;
                }

                // tutaj odbiór z socketu:
                // result = self.recv_rip_message() => {
                //     ...
                // }
            }
        }
    }
}
