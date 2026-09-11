use socket2 as s2;
use std::net::{Ipv4Addr, SocketAddrV4};

use crate::common::ifc_nametoindex;
use crate::result::*;
use std::io;
use std::str::FromStr;

pub struct RipSocket {
    pub socket: tokio::net::UdpSocket,
    pub if_name: String,
    pub if_index: u32,
}

impl RipSocket {
    pub fn new_rx_socket(if_name: &str) -> io::Result<Self> {
        let if_index = ifc_nametoindex(if_name)?;
        let socket = create_multicast_rx_socket(if_name, if_index)?;

        Ok(Self {
            socket: socket,
            if_name: if_name.to_string(),
            if_index,
        })
    }

    pub fn new_tx_socket(if_name: &str) -> io::Result<Self> {
        let if_index = ifc_nametoindex(if_name)?;
        let socket = create_multicast_tx_socket(if_name)?;

        Ok(Self {
            socket: socket,
            if_name: if_name.to_string(),
            if_index,
        })
    }

    pub async fn send_multicast(&self, buffer: &[u8]) -> io::Result<()> {
        let addr = SocketAddrV4::new(
            Ipv4Addr::from_str(RIP_MULTICAST_ADDR).unwrap(),
            RIP_UDP_PORT,
        );

        let sentn = self.socket.send_to(buffer, addr).await?;
        if sentn != buffer.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to send full RIP packet",
            ));
        }

        Ok(())
    }
}

fn into_tokio_socket(socket: s2::Socket) -> io::Result<tokio::net::UdpSocket> {
    let std_socket: std::net::UdpSocket = socket.into();
    let socket = tokio::net::UdpSocket::from_std(std_socket)?;

    Ok(socket)
}

fn bind_port_and_device(socket: &s2::Socket, if_name: &str) -> io::Result<()> {
    socket.bind_device(Some(if_name.as_bytes()))?;
    socket.set_reuse_port(true)?;

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, RIP_UDP_PORT);
    let addr = s2::SockAddr::from(addr);
    socket.bind(&addr)?;
    Ok(())
}

fn create_multicast_rx_socket(if_name: &str, if_index: u32) -> io::Result<tokio::net::UdpSocket> {
    let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP))?;
    bind_port_and_device(&socket, if_name)?;
    socket.set_nonblocking(true)?;

    let ifc_index = s2::InterfaceIndexOrAddress::Index(if_index);
    socket.join_multicast_v4_n(
        &Ipv4Addr::from_str(RIP_MULTICAST_ADDR).ok().unwrap(),
        &ifc_index,
    )?;

    into_tokio_socket(socket)
}

fn create_multicast_tx_socket(if_name: &str) -> io::Result<tokio::net::UdpSocket> {
    let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP))?;
    bind_port_and_device(&socket, if_name)?;
    socket.set_nonblocking(true)?;
    socket.set_multicast_loop_v4(false)?;

    into_tokio_socket(socket)
}
