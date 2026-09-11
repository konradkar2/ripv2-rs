use std::io::{self};
use std::net::{Ipv4Addr, SocketAddrV4};

use socket2 as s2;

use crate::result::*;
use crate::rip_database::RipDatabase;
use crate::rip_packet::*;
use crate::rip_socket::RipSocket;
use std::{mem::size_of, slice};

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) }
}

fn create_unicast_tx_socket(if_name: &str) -> io::Result<tokio::net::UdpSocket> {
    let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP))?;
    socket.bind_device(Some(if_name.as_bytes()))?;
    socket.set_reuse_port(true)?;

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, RIP_UDP_PORT);
    socket.bind(&s2::SockAddr::from(addr))?;
    socket.set_nonblocking(true)?;

    let std_socket: std::net::UdpSocket = socket.into();
    tokio::net::UdpSocket::from_std(std_socket)
}

fn build_response_buffer(database: &RipDatabase, target_if_index: u32) -> Option<Vec<u8>> {
    let change_only = false;
    let mut entries: Vec<RipEntry> = database
        .get_routes_for_advertisement(target_if_index, change_only)
        .collect();

    if entries.is_empty() {
        return None;
    }

    let header = RipHeader {
        command: RIP_CMD_RESPONSE,
        version: RIP_2_VERSION,
        padding: 0,
    };

    let mut buffer = vec![0; RIP_HEADER_SIZE + entries.len() * RIP_ENTRY_SIZE];
    buffer[..RIP_HEADER_SIZE].copy_from_slice(as_bytes::<RipHeader>(&header));

    for (index, entry) in entries.iter_mut().enumerate() {
        entry.to_be();
        let offset = RIP_HEADER_SIZE + index * RIP_ENTRY_SIZE;
        buffer[offset..offset + RIP_ENTRY_SIZE].copy_from_slice(as_bytes::<RipEntry>(entry));
    }

    Some(buffer)
}

pub struct RipUpdater {}

impl RipUpdater {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn rip_send_request_multicast(socket: &RipSocket) -> io::Result<()> {
        let header = RipHeader {
            command: RIP_CMD_REQUEST,
            version: RIP_2_VERSION,
            padding: 0,
        };
        let mut entry = RIP_ENTRY_REQUEST;
        entry.to_be();

        let mut buffer = vec![0; RIP_HEADER_SIZE + RIP_ENTRY_SIZE];
        buffer[..RIP_HEADER_SIZE].copy_from_slice(as_bytes::<RipHeader>(&header));
        buffer[RIP_HEADER_SIZE..].copy_from_slice(as_bytes::<RipEntry>(&entry));

        socket.send_multicast(&buffer).await?;

        Ok(())
    }

    pub async fn rip_send_response_unicast(
        &self,
        database: &RipDatabase,
        target: &SocketAddrV4,
        target_if_info: &RipIfInfo,
    ) -> io::Result<()> {
        let Some(buffer) = build_response_buffer(database, target_if_info.if_index) else {
            return Ok(());
        };

        let socket = create_unicast_tx_socket(&target_if_info.if_name)?;
        let sentn = socket.send_to(&buffer, *target).await?;
        if sentn != buffer.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to send full RIP response",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::AF_INET;

    fn rip_entry(ip_address: Ipv4Addr, subnet_mask: Ipv4Addr, next_hop: Ipv4Addr) -> RipEntry {
        RipEntry {
            routing_family_id: AF_INET as u16,
            route_tag: 0,
            ip_address: u32::from(ip_address),
            subnet_mask: u32::from(subnet_mask),
            next_hop: u32::from(next_hop),
            metric: 1,
        }
    }

    #[test]
    fn response_buffer_contains_advertised_routes() {
        let mut database = RipDatabase::new();
        let target_if_index = 2;
        let advertised_entry = rip_entry(
            Ipv4Addr::new(10, 0, 1, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(10, 0, 0, 1),
        );
        let split_horizon_entry = rip_entry(
            Ipv4Addr::new(10, 0, 2, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::UNSPECIFIED,
        );

        database.add_local_route(advertised_entry, 1).unwrap();
        database
            .add_local_route(split_horizon_entry, target_if_index)
            .unwrap();

        let buffer = build_response_buffer(&database, target_if_index).expect("response buffer");
        let packet = RipPacketData::from_slice(&buffer).unwrap();

        assert_eq!(packet.header.command, RIP_CMD_RESPONSE);
        assert_eq!(packet.header.version, RIP_2_VERSION);
        assert_eq!(packet.entries.len(), 1);
        assert_eq!(
            packet.entries[0].routing_family_id,
            advertised_entry.routing_family_id
        );
        assert_eq!(packet.entries[0].ip_address, advertised_entry.ip_address);
        assert_eq!(packet.entries[0].subnet_mask, advertised_entry.subnet_mask);
        assert_eq!(packet.entries[0].next_hop, 0);
        assert_eq!(packet.entries[0].metric, advertised_entry.metric);
    }

    #[test]
    fn response_buffer_is_empty_when_no_routes_can_be_advertised() {
        let mut database = RipDatabase::new();
        let entry = rip_entry(
            Ipv4Addr::new(10, 0, 1, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::UNSPECIFIED,
        );

        database.add_local_route(entry, 1).unwrap();

        assert!(build_response_buffer(&database, 1).is_none());
    }
}
