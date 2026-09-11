use std::{net::Ipv4Addr, str::FromStr};

use libc::AF_INET;

use crate::{
    address::prefix_to_mask,
    cfg::AdvertisedNetwork,
    common::ifc_nametoindex,
    result::{RipError, RipResult},
    rip_packet::RipEntry,
};

pub struct LocalRoute {
    pub entry: RipEntry,
    pub if_index: u32,
}

pub fn advertised_network_to_local_route(network: &AdvertisedNetwork) -> RipResult<LocalRoute> {
    let if_index = ifc_nametoindex(&network.dev).map_err(|err| {
        RipError::InvalidConfiguration(format!(
            "invalid dev: {}: {}",
            network.address.as_str(),
            err
        ))
    })?;
    let subnet_mask = prefix_to_mask(network.prefix)?;
    let ip_address = Ipv4Addr::from_str(network.address.as_str()).map_err(|err| {
        RipError::InvalidConfiguration(format!(
            "invalid address: {}: {}",
            network.address.as_str(),
            err
        ))
    })?;
    let ip_address = u32::from(ip_address) & subnet_mask;

    let local_entry = RipEntry {
        routing_family_id: AF_INET as u16,
        route_tag: 0,
        ip_address,
        subnet_mask,
        next_hop: 0,
        metric: 1,
    };

    Ok(LocalRoute {
        entry: local_entry,
        if_index,
    })
}
