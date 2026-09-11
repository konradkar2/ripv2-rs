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

    let changed_only = false;
    let buffer =
        build_response_buffer(&database, target_if_index, changed_only).expect("response buffer");
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

    let target_if_index = 1;
    let changed_only = false;

    assert!(build_response_buffer(&database, target_if_index, changed_only).is_none());
}

#[test]
fn changed_only_response_buffer_contains_only_changed_routes() {
    let mut database = RipDatabase::new();
    let target_if_index = 3;
    let unchanged_entry = rip_entry(
        Ipv4Addr::new(10, 0, 1, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::UNSPECIFIED,
    );
    let changed_entry = rip_entry(
        Ipv4Addr::new(10, 0, 2, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(10, 0, 0, 2),
    );

    database.add_local_route(unchanged_entry, 1).unwrap();
    database.add_remote_route(changed_entry, 2).unwrap();

    let changed_only = true;
    let buffer =
        build_response_buffer(&database, target_if_index, changed_only).expect("response buffer");
    let packet = RipPacketData::from_slice(&buffer).unwrap();

    assert_eq!(packet.entries.len(), 1);
    assert_eq!(packet.entries[0].ip_address, changed_entry.ip_address);
}
