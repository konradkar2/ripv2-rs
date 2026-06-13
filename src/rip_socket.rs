use socket2 as s2;
use std::net::{Ipv4Addr, SocketAddrV4};

use std::io;
use std::str::FromStr;

const RIP_MULTICAST_ADDR: &str = "224.0.0.9";
const RIP_UDP_PORT: u16 = 520;

pub struct RipSocket {
    socket: s2::Socket,
    dev: String,
    if_index: u32,
}

fn ifc_nametoindex(if_name: &str) -> io::Result<u32> {
    if if_name.len() >= (libc::IFNAMSIZ - 1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid interface name: {}", if_name),
        ));
    }

    let mut buffer: Vec<libc::c_char> = vec![0 as libc::c_char; libc::IFNAMSIZ];

    for (dst, src) in buffer.iter_mut().zip(if_name.as_bytes()) {
        *dst = *src as libc::c_char;
    }

    let if_index = unsafe { libc::if_nametoindex(buffer.as_ptr()) };
    if if_index == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(if_index)
}

impl RipSocket {
    pub fn create(if_name: &str) -> io::Result<Self> {
        let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP))?;
        let if_index = ifc_nametoindex(if_name)?;

        let socket = Self {
            socket,
            dev: if_name.to_string(),
            if_index,
        };
        Ok(socket)
    }

    fn bind_port_and_device(&self) -> io::Result<()> {
        self.socket.bind_device(Some(self.dev.as_bytes()))?;
        self.socket.set_reuse_port(true)?;

        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, RIP_UDP_PORT);
        let addr = s2::SockAddr::from(addr);
        self.socket.bind(&addr)?;
        Ok(())
    }

    pub fn configure_as_multicast_rx(&self) -> io::Result<()> {
        self.bind_port_and_device()?;
        self.socket.set_nonblocking(true)?;

        let ifc_index = s2::InterfaceIndexOrAddress::Index(self.if_index);
        self.socket.join_multicast_v4_n(
            &Ipv4Addr::from_str(RIP_MULTICAST_ADDR).ok().unwrap(),
            &ifc_index,
        )?;

        Ok(())
    }

    pub fn configure_as_multicast_tx(&self) -> io::Result<()> {
        self.bind_port_and_device()?;
        self.socket.set_multicast_loop_v4(false)?;

        Ok(())
    }
}
