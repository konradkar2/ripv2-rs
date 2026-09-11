use crate::result::RipError;
use crate::result::RipResult;
use crate::rip_socket::RipSocket;
use std::io;

pub struct RipIfc {
    pub if_name: String,
    pub rx: Option<RipSocket>,
    pub tx: RipSocket,
}

impl RipIfc {
    fn create_ifc(if_name: &str) -> io::Result<RipIfc> {
        let socket_tx = RipSocket::new_tx_socket(if_name)?;
        let socket_rx = RipSocket::new_rx_socket(if_name)?;

        Ok(Self {
            if_name: if_name.to_string(),
            rx: Some(socket_rx),
            tx: socket_tx,
        })
    }
    pub fn create(if_name: &str) -> RipResult<RipIfc> {
        let ifc =
            RipIfc::create_ifc(if_name).map_err(|err| RipError::FailedToConfigureInterface {
                msg: err.to_string(),
                if_name: if_name.to_string(),
            })?;

        Ok(ifc)
    }
}
