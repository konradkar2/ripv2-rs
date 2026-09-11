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
        log::info!("adding route to kernel: {}", format_route(route));
        let route = build_route_message(route);
        self.handle
            .route()
            .add(route)
            .execute()
            .await
            .map_err(|err| RipError::IoError(format!("failed to add route: {}", err)))
    }

    async fn delete_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        log::info!("deleting route from kernel: {}", format_route(route));
        let route = build_route_message(route);
        self.handle
            .route()
            .del(route)
            .execute()
            .await
            .map_err(|err| RipError::IoError(format!("failed to delete route: {}", err)))
    }
}

fn format_route(route: &RipDbEntry) -> String {
    let entry = route.rip_entry;

    format!(
        "{}/{} via {} if_index {} metric {}",
        Ipv4Addr::from(entry.ip_address),
        Ipv4Addr::from(entry.subnet_mask),
        Ipv4Addr::from(entry.next_hop),
        route.if_index,
        entry.metric
    )
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
#[path = "tests/routing_table_rtnetlink_tests.rs"]
mod tests;
