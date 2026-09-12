use super::*;
use libc::AF_INET;

fn rip_entry(ip_address: Ipv4Addr, next_hop: Ipv4Addr) -> RipEntry {
    RipEntry {
        routing_family_id: AF_INET as u16,
        route_tag: 0,
        ip_address: u32::from(ip_address),
        subnet_mask: u32::from(Ipv4Addr::new(255, 255, 255, 0)),
        next_hop: u32::from(next_hop),
        metric: 2,
    }
}

#[test]
fn timed_out_route_moves_to_garbage_and_is_advertised_as_changed() {
    let mut database = RipDatabase::new();
    let if_index = 2;
    let entry = rip_entry(Ipv4Addr::new(10, 0, 1, 0), Ipv4Addr::new(10, 0, 0, 1));

    database.add_remote_route(entry, if_index).unwrap();

    let timeout_increment_secs = 180;
    let timeout_limit_secs = 180;
    let timed_out_routes =
        database.collect_timed_out_routes(timeout_increment_secs, timeout_limit_secs);
    assert_eq!(timed_out_routes.len(), 1);

    let garbage_started_at = Instant::now();
    let garbage_route = database
        .move_route_to_garbage(&entry, garbage_started_at)
        .unwrap();

    assert!(database.ok_routes.is_empty());
    assert_eq!(database.garbage_routes.len(), 1);
    assert!(database.any_route_changed());
    assert!(garbage_route.changed);
    assert!(!garbage_route.in_routing_table);
    assert_eq!(garbage_route.rip_entry.metric, 16);

    let target_if_index = 3;
    let changed_only = true;
    let advertised_routes: Vec<RipEntry> = database
        .get_routes_for_advertisement(target_if_index, changed_only)
        .collect();

    assert_eq!(advertised_routes.len(), 1);
    assert_eq!(advertised_routes[0].metric, 16);
}

#[test]
fn route_key_identifies_destination_not_next_hop() {
    let mut database = RipDatabase::new();
    let first_if_index = 2;
    let second_if_index = 3;
    let first_entry = rip_entry(Ipv4Addr::new(10, 0, 1, 0), Ipv4Addr::new(10, 0, 0, 1));
    let second_entry = rip_entry(Ipv4Addr::new(10, 0, 1, 0), Ipv4Addr::new(10, 0, 0, 2));

    let added_route = database
        .add_remote_route(first_entry, first_if_index)
        .unwrap();
    let duplicate_result = database.add_remote_route(second_entry, second_if_index);

    assert!(duplicate_result.is_err());
    assert_eq!(database.ok_routes.len(), 1);
    assert_eq!(added_route.rip_entry.next_hop, first_entry.next_hop);
    assert_eq!(added_route.if_index, first_if_index);

    let route = database
        .get_route(&second_entry)
        .expect("route by destination");

    assert_eq!(route.rip_entry.next_hop, first_entry.next_hop);
    assert_eq!(route.if_index, first_if_index);
}

#[test]
fn garbage_collection_removes_expired_garbage_routes() {
    let mut database = RipDatabase::new();
    let if_index = 2;
    let entry = rip_entry(Ipv4Addr::new(10, 0, 1, 0), Ipv4Addr::new(10, 0, 0, 1));

    database.add_remote_route(entry, if_index).unwrap();
    database
        .move_route_to_garbage(&entry, Instant::now())
        .unwrap();

    let now = Instant::now() + Duration::from_secs(121);
    let garbage_lifetime = Duration::from_secs(120);
    let removed_routes = database.remove_expired_garbage_routes(now, garbage_lifetime);

    assert_eq!(removed_routes.len(), 1);
    assert_eq!(removed_routes[0].rip_entry.ip_address, entry.ip_address);
    assert_eq!(removed_routes[0].rip_entry.metric, 16);
    assert!(database.garbage_routes.is_empty());
}

#[test]
fn poison_all_routes_marks_all_routes_unreachable() {
    let mut database = RipDatabase::new();
    let local_if_index = 1;
    let remote_if_index = 2;
    let local_entry = rip_entry(Ipv4Addr::new(10, 0, 1, 0), Ipv4Addr::UNSPECIFIED);
    let remote_entry = rip_entry(Ipv4Addr::new(10, 0, 2, 0), Ipv4Addr::new(10, 0, 0, 2));

    database
        .add_local_route(local_entry, local_if_index)
        .unwrap();
    database
        .add_remote_route(remote_entry, remote_if_index)
        .unwrap();
    database.mark_all_routes_as_unchanged();

    database.poison_all_routes();

    assert!(database.any_route_changed());
    assert!(
        database
            .ok_routes
            .values()
            .all(|route| route.changed && route.rip_entry.metric == 16)
    );
}
