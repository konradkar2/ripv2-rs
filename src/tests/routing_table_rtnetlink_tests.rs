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
