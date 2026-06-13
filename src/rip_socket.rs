
use socket2 as s2;
use std::net::{Ipv4Addr, SocketAddrV4};

use std::io;
use std::str::FromStr;

const RIP_MULTICAST_ADDR: &str = "224.0.0.9";
const RIP_UDP_PORT: u16 = 520;

struct RipSocket {
    socket: s2::Socket,
    dev: String,
    if_index: u32,
}

fn ifc_nametoindex(if_name: &str) -> io::Result<u32> {
    if if_name.len() >= (libc::IFNAMSIZ -1 ) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid interface name"));
    }

    let mut buffer: Vec<libc::c_char> = vec![0 as libc::c_char; libc::IFNAMSIZ];

    for (dst, src) in buffer.iter_mut().zip(if_name.as_bytes()) {
        *dst = *src as libc::c_char;
    }

    let if_index = unsafe {
        libc::if_nametoindex(buffer.as_ptr())
    };

    if if_index == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(if_index) 
}

impl RipSocket {
    pub fn create_and_configure(if_name: &str) -> io::Result<Self> {
        let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP ))?;
        let if_index = ifc_nametoindex(if_name)?;

        Ok(Self { socket, dev: if_name.to_string(), if_index})
    }

    fn configure(&self) -> io::Result<()> {
        self.socket.set_nonblocking(true)?;
        self.socket.bind_device(Some(self.dev.as_bytes()))?;
        self.socket.set_reuse_port(true)?;

        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, RIP_UDP_PORT);
        let addr = s2::SockAddr::from(addr);
        self.socket.bind(&addr)?;
        
        let interface = s2::InterfaceIndexOrAddress::Index(self.if_index);
        self.socket.join_multicast_v4_n(&Ipv4Addr::from_str(RIP_MULTICAST_ADDR).ok().unwrap(), &interface)?;

        Ok(())
    }
}
