use std::net::Ipv4Addr;
use std::time::Instant;

use crate::http::HttpRoute;
use crate::rip_database::RipDatabase;
use crate::rip_packet::RipEntry;

fn route_entry(address: &str, next_hop: &str, metric: u32) -> RipEntry {
    RipEntry {
        routing_family_id: 2,
        route_tag: 0,
        ip_address: u32::from(address.parse::<Ipv4Addr>().unwrap()),
        subnet_mask: u32::from(Ipv4Addr::new(255, 255, 255, 0)),
        next_hop: u32::from(next_hop.parse::<Ipv4Addr>().unwrap()),
        metric,
    }
}

#[test]
fn http_routes_snapshot_contains_active_and_garbage_routes() {
    let mut database = RipDatabase::new();
    let local_if_index = 10;
    let remote_if_index = 20;
    let local_if_name = "lan-test";
    let remote_if_name = "rip-test";
    let local_route = route_entry("10.1.1.0", "0.0.0.0", 1);
    let remote_route = route_entry("10.2.2.0", "10.0.12.2", 3);

    database
        .add_local_route(local_route, local_if_index, local_if_name)
        .expect("local route should be added");
    database
        .add_remote_route(remote_route, remote_if_index, remote_if_name)
        .expect("remote route should be added");
    database
        .move_route_to_garbage(&remote_route, Instant::now())
        .expect("remote route should move to garbage");

    let routes = HttpRoute::from_database(&database);

    assert_eq!(routes.len(), 2);
    assert!(routes.iter().any(|route| {
        route.destination == "10.1.1.0"
            && route.prefix == 24
            && route.interface_name == "lan-test"
            && route.route_type == "local"
            && route.state == "active"
            && !route.in_kernel
    }));
    assert!(routes.iter().any(|route| {
        route.destination == "10.2.2.0"
            && route.prefix == 24
            && route.next_hop == "10.0.12.2"
            && route.metric == 16
            && route.interface_name == "rip-test"
            && route.route_type == "remote"
            && route.state == "garbage"
            && !route.in_kernel
    }));
}
