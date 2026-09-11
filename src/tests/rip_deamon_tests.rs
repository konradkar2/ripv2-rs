use super::*;
use crate::result::RIP_CMD_RESPONSE;
use crate::routing_table_stub::StubRoutingTableDriver;
use libc::AF_INET;
use std::net::{Ipv4Addr, SocketAddrV4};

fn if_info(if_index: u32) -> RipIfInfo {
    RipIfInfo {
        if_name: "eth-test".to_string(),
        if_index,
    }
}

fn response_entry(metric: u32) -> RipEntry {
    RipEntry {
        routing_family_id: AF_INET as u16,
        route_tag: 0,
        ip_address: u32::from(Ipv4Addr::new(10, 0, 1, 0)),
        subnet_mask: u32::from(Ipv4Addr::new(255, 255, 255, 0)),
        next_hop: 0,
        metric,
    }
}

fn response_packet(entry: RipEntry, source: SocketAddrV4, if_index: u32) -> RipPacket {
    RipPacket {
        data: RipPacketData {
            header: crate::rip_packet::RipHeader {
                command: RIP_CMD_RESPONSE,
                version: RIP_2_VERSION,
                padding: 0,
            },
            entries: vec![entry],
        },
        if_info: if_info(if_index),
        source,
    }
}

fn learned_route_entry(entry: RipEntry, source_addr: Ipv4Addr) -> RipEntry {
    let mut route_entry = entry;
    route_entry.metric = update_metric(route_entry.metric);
    route_entry.next_hop = u32::from(source_addr);
    route_entry
}

fn test_deamon() -> RipDeamon<StubRoutingTableDriver> {
    RipDeamon::with_routing_table(RoutingTable::with_driver(StubRoutingTableDriver::new()))
}

#[test]
fn request_warmup_duration_uses_startup_jitter_range() {
    let warmup_duration = create_request_warmup_duration();
    let warmup_millis = warmup_duration.as_millis() as u64;

    assert!(warmup_millis >= RIP_REQUEST_WARMUP_MIN_MILLIS);
    assert!(warmup_millis <= RIP_REQUEST_WARMUP_MAX_MILLIS);
}

#[tokio::test]
async fn response_adds_new_route_to_database() {
    let mut deamon = test_deamon();
    let if_index = 2;
    let source_addr = Ipv4Addr::new(10, 0, 0, 2);
    let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
    let entry = response_entry(1);
    let expected_route = learned_route_entry(entry, source_addr);

    deamon
        .handle_packet(response_packet(entry, source, if_index))
        .await
        .unwrap();

    let route = deamon
        .database
        .get_route(&expected_route, if_index)
        .expect("learned route");

    assert_eq!(route.rip_entry, expected_route);
    assert!(route.changed);
    assert!(!route.is_local);
    assert!(route.in_routing_table);

    let routing_driver = deamon.routing_table.driver();
    assert_eq!(routing_driver.added_routes.len(), 1);
    assert_eq!(routing_driver.added_routes[0].rip_entry, expected_route);
    assert!(routing_driver.deleted_routes.is_empty());
}

#[tokio::test]
async fn response_replaces_existing_route_when_new_metric_is_better() {
    let mut deamon = test_deamon();
    let if_index = 2;
    let source_addr = Ipv4Addr::new(10, 0, 0, 2);
    let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
    let first_entry = response_entry(5);
    let second_entry = response_entry(1);
    let expected_route = learned_route_entry(second_entry, source_addr);

    deamon
        .handle_packet(response_packet(first_entry, source, if_index))
        .await
        .unwrap();
    deamon
        .handle_packet(response_packet(second_entry, source, if_index))
        .await
        .unwrap();

    assert_eq!(deamon.database.ok_routes.len(), 1);

    let route = deamon
        .database
        .get_route(&expected_route, if_index)
        .expect("replaced route");

    assert_eq!(route.rip_entry, expected_route);

    let routing_driver = deamon.routing_table.driver();
    assert_eq!(routing_driver.added_routes.len(), 2);
    assert_eq!(routing_driver.deleted_routes.len(), 1);
    assert_eq!(routing_driver.added_routes[1].rip_entry, expected_route);
    assert_eq!(
        routing_driver.deleted_routes[0].rip_entry,
        learned_route_entry(first_entry, source_addr)
    );
}

#[tokio::test]
async fn response_keeps_existing_route_when_new_metric_is_worse() {
    let mut deamon = test_deamon();
    let if_index = 2;
    let source_addr = Ipv4Addr::new(10, 0, 0, 2);
    let source = SocketAddrV4::new(source_addr, RIP_UDP_PORT);
    let first_entry = response_entry(1);
    let second_entry = response_entry(5);
    let expected_route = learned_route_entry(first_entry, source_addr);
    let ignored_route = learned_route_entry(second_entry, source_addr);

    deamon
        .handle_packet(response_packet(first_entry, source, if_index))
        .await
        .unwrap();
    deamon
        .handle_packet(response_packet(second_entry, source, if_index))
        .await
        .unwrap();

    assert_eq!(deamon.database.ok_routes.len(), 1);

    let route = deamon
        .database
        .get_route(&expected_route, if_index)
        .expect("existing route");

    assert_eq!(route.rip_entry, expected_route);
    assert_ne!(route.rip_entry.metric, ignored_route.metric);

    let routing_driver = deamon.routing_table.driver();
    assert_eq!(routing_driver.added_routes.len(), 1);
    assert_eq!(routing_driver.deleted_routes.len(), 0);
}
