use crate::address::{is_net_mask_valid, is_unicast_address};
use crate::cfg::{AdvertisedNetwork, RipConfiguration};
use crate::result::{self, RIP_2_VERSION, RIP_UDP_PORT};
use crate::result::{RipError, RipResult};
use crate::rip_database::RipDatabase;
use crate::rip_ifc::RipIfc;
use crate::rip_packet::{RipEntry, RipIfInfo, RipPacket, RipPacketData};
use crate::rip_route::advertised_network_to_local_route;
use crate::rip_socket::RipSocket;
use crate::rip_updater::RipUpdater;
use crate::routing_table::RoutingTable;
use std::future;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Sleep};

const RIP_INFINITY_METRIC: u32 = 16;

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

fn is_metric_valid(metric: u32) -> bool {
    (1..=RIP_INFINITY_METRIC).contains(&metric)
}

fn is_entry_valid(entry: &RipEntry) -> bool {
    is_unicast_address(entry.ip_address)
        && is_net_mask_valid(entry.subnet_mask)
        && is_metric_valid(entry.metric)
}

fn update_metric(metric: u32) -> u32 {
    metric.saturating_add(1).min(RIP_INFINITY_METRIC)
}

fn response_entry_to_route(entry: &RipEntry, packet: &RipPacket) -> Option<RipEntry> {
    if !is_entry_valid(entry) {
        eprintln!("Invalid RIP response entry: {:?}", entry);
        return None;
    }

    let mut route_entry = *entry;
    route_entry.metric = update_metric(route_entry.metric);
    route_entry.next_hop = u32::from(*packet.source.ip());

    Some(route_entry)
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
        } else if packet.is_response() {
            self.handle_response_packet(&packet)?;
        } else {
            return Err(RipError::MalformedPacket());
        }

        Ok(())
    }

    fn handle_response_packet(&mut self, packet: &RipPacket) -> result::RipResult<()> {
        if packet.data.header.version != RIP_2_VERSION {
            return Err(RipError::MalformedPacket());
        }

        if packet.source.port() != RIP_UDP_PORT {
            eprintln!("Ignoring RIP response from non-RIP port: {}", packet.source);
            return Ok(());
        }

        for entry in packet.data.entries.iter() {
            let Some(route_entry) = response_entry_to_route(entry, packet) else {
                continue;
            };

            self.update_route_from_response(route_entry, packet.if_info.if_index)?;
        }

        Ok(())
    }

    fn update_route_from_response(
        &mut self,
        route_entry: RipEntry,
        if_index: u32,
    ) -> result::RipResult<()> {
        let old_route = self.database.get_route(&route_entry, if_index).cloned();

        match old_route {
            None => {
                let new_route = self.database.add_remote_route(route_entry, if_index)?;
                self.routing_table.add_route(&new_route)?;
            }
            Some(old_route) if old_route.rip_entry.metric > route_entry.metric => {
                self.routing_table.delete_route(&old_route)?;
                self.database
                    .remove_route(&old_route.rip_entry, old_route.if_index)?;

                let new_route = self.database.add_remote_route(route_entry, if_index)?;
                self.routing_table.add_route(&new_route)?;
            }
            Some(_) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::RIP_CMD_RESPONSE;
    use libc::AF_INET;
    use std::net::{Ipv4Addr, SocketAddrV4};

    fn if_info(if_index: u32) -> RipIfInfo {
        RipIfInfo {
            if_name: "eth-test".to_string(),
            if_index,
        }
    }

    fn response_entry(metric: u32) -> RipEntry {
        RipEntry {
            routing_family_id: AF_INET as u16,
            route_tag: 0,
            ip_address: u32::from(Ipv4Addr::new(10, 0, 1, 0)),
            subnet_mask: u32::from(Ipv4Addr::new(255, 255, 255, 0)),
            next_hop: 0,
            metric,
        }
    }

    fn response_packet(entry: RipEntry, source: SocketAddrV4, if_index: u32) -> RipPacket {
        RipPacket {
            data: RipPacketData {
                header: crate::rip_packet::RipHeader {
                    command: RIP_CMD_RESPONSE,
                    version: RIP_2_VERSION,
                    padding: 0,
                },
                entries: vec![entry],
            },
            if_info: if_info(if_index),
            source,
        }
    }

    fn learned_route_entry(entry: RipEntry, source_addr: Ipv4Addr) -> RipEntry {
        let mut route_entry = entry;
        route_entry.metric = update_metric(route_entry.metric);
        route_entry.next_hop = u32::from(source_addr);
        route_entry
    }

    #[tokio::test]
    async fn response_adds_new_route_to_database() {
        let mut deamon = RipDeamon::new();
        let if_index = 2;
        let source_addr = Ipv4Addr::new(10, 0, 0, 2);
        let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
        let entry = response_entry(1);
        let expected_route = learned_route_entry(entry, source_addr);

        deamon
            .handle_packet(response_packet(entry, source, if_index))
            .await
            .unwrap();

        let route = deamon
            .database
            .get_route(&expected_route, if_index)
            .expect("learned route");

        assert_eq!(route.rip_entry, expected_route);
        assert!(route.changed);
        assert!(!route.is_local);
        assert!(route.in_routing_table);
    }

    #[tokio::test]
    async fn response_replaces_existing_route_when_new_metric_is_better() {
        let mut deamon = RipDeamon::new();
        let if_index = 2;
        let source_addr = Ipv4Addr::new(10, 0, 0, 2);
        let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
        let first_entry = response_entry(5);
        let second_entry = response_entry(1);
        let expected_route = learned_route_entry(second_entry, source_addr);

        deamon
            .handle_packet(response_packet(first_entry, source, if_index))
            .await
            .unwrap();
        deamon
            .handle_packet(response_packet(second_entry, source, if_index))
            .await
            .unwrap();

        assert_eq!(deamon.database.ok_routes.len(), 1);

        let route = deamon
            .database
            .get_route(&expected_route, if_index)
            .expect("replaced route");

        assert_eq!(route.rip_entry, expected_route);
    }

    #[tokio::test]
    async fn response_keeps_existing_route_when_new_metric_is_worse() {
        let mut deamon = RipDeamon::new();
        let if_index = 2;
        let source_addr = Ipv4Addr::new(10, 0, 0, 2);
        let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
        let first_entry = response_entry(1);
        let second_entry = response_entry(5);
        let expected_route = learned_route_entry(first_entry, source_addr);
        let ignored_route = learned_route_entry(second_entry, source_addr);

        deamon
            .handle_packet(response_packet(first_entry, source, if_index))
            .await
            .unwrap();
        deamon
            .handle_packet(response_packet(second_entry, source, if_index))
            .await
            .unwrap();

        assert_eq!(deamon.database.ok_routes.len(), 1);

        let route = deamon
            .database
            .get_route(&expected_route, if_index)
            .expect("existing route");

        assert_eq!(route.rip_entry, expected_route);
        assert_ne!(route.rip_entry.metric, ignored_route.metric);
    }
}
