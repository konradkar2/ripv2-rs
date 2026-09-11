use std::net::Ipv4Addr;

use rtnetlink::{
    Handle, RouteMessageBuilder, new_connection,
    packet_route::route::{RouteMessage, RouteProtocol},
};

use crate::result::{RipError, RipResult};
use crate::rip_database::RipDbEntry;
use crate::routing_table::RoutingTableDriver;

const RIP_ROUTE_PRIORITY_BASE: u32 = 20;

pub struct RtNetlinkRoutingTableDriver {
    handle: Handle,
}

impl RtNetlinkRoutingTableDriver {
    pub async fn new() -> RipResult<Self> {
        let (connection, handle, _) = new_connection().map_err(|err| {
            RipError::IoError(format!("failed to create rtnetlink connection: {}", err))
        })?;

        tokio::spawn(connection);

        Ok(Self { handle })
    }
}

impl RoutingTableDriver for RtNetlinkRoutingTableDriver {
    async fn add_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        let route = build_route_message(route);
        self.handle
            .route()
            .add(route)
            .execute()
            .await
            .map_err(|err| RipError::IoError(format!("failed to add route: {}", err)))
    }

    async fn delete_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        let route = build_route_message(route);
        self.handle
            .route()
            .del(route)
            .execute()
            .await
            .map_err(|err| RipError::IoError(format!("failed to delete route: {}", err)))
    }
}

fn build_route_message(route: &RipDbEntry) -> RouteMessage {
    let entry = route.rip_entry;
    let destination = Ipv4Addr::from(entry.ip_address);
    let prefix_len = entry.subnet_mask.leading_ones() as u8;
    let gateway = Ipv4Addr::from(entry.next_hop);
    let priority = RIP_ROUTE_PRIORITY_BASE + entry.metric;

    RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(destination, prefix_len)
        .gateway(gateway)
        .output_interface(route.if_index)
        .priority(priority)
        .protocol(RouteProtocol::Rip)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtnetlink::packet_route::route::{RouteAddress, RouteAttribute};

    use crate::rip_packet::RipEntry;

    fn route_entry() -> RipDbEntry {
        RipDbEntry {
            rip_entry: RipEntry {
                routing_family_id: libc::AF_INET as u16,
                route_tag: 0,
                ip_address: u32::from(Ipv4Addr::new(10, 0, 1, 0)),
                subnet_mask: u32::from(Ipv4Addr::new(255, 255, 255, 0)),
                next_hop: u32::from(Ipv4Addr::new(10, 0, 0, 1)),
                metric: 2,
            },
            if_index: 7,
            changed: true,
            is_local: false,
            in_routing_table: true,
            timeout_cnt: 0,
        }
    }

    #[test]
    fn route_message_uses_rip_protocol_and_route_attributes() {
        let route = route_entry();
        let message = build_route_message(&route);

        assert_eq!(message.header.protocol, RouteProtocol::Rip);
        assert_eq!(message.header.destination_prefix_length, 24);

        assert!(message.attributes.iter().any(|attribute| {
            matches!(
                attribute,
                RouteAttribute::Destination(RouteAddress::Inet(address))
                    if *address == Ipv4Addr::new(10, 0, 1, 0)
            )
        }));
        assert!(message.attributes.iter().any(|attribute| {
            matches!(
                attribute,
                RouteAttribute::Gateway(RouteAddress::Inet(address))
                    if *address == Ipv4Addr::new(10, 0, 0, 1)
            )
        }));
        assert!(
            message
                .attributes
                .iter()
                .any(|attribute| matches!(attribute, RouteAttribute::Oif(7)))
        );
        assert!(
            message
                .attributes
                .iter()
                .any(|attribute| matches!(attribute, RouteAttribute::Priority(22)))
        );
    }
}
