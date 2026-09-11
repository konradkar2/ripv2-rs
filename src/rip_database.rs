use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::net::Ipv4Addr;
use std::time::Instant;

use crate::result::{RipError, RipResult};
use crate::rip_packet::RipEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RipRouteKey {
    pub if_index: u32,
    pub ip_address: u32,
    pub subnet_mask: u32,
    pub next_hop: u32,
}

impl fmt::Display for RipRouteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "if_index={}, ip_address={}, subnet_mask={}, next_hop={}",
            self.if_index,
            Ipv4Addr::from(self.ip_address),
            Ipv4Addr::from(self.subnet_mask),
            Ipv4Addr::from(self.next_hop),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RipDbEntry {
    pub rip_entry: RipEntry,
    pub if_index: u32,

    pub changed: bool,
    pub is_local: bool,
    pub in_routing_table: bool,
    pub timeout_cnt: u16,
}

pub struct RipDatabase {
    pub ok_routes: HashMap<RipRouteKey, RipDbEntry>,
    //garbage_routes: HashMap<RipRouteKey, RipRouteDescription>,
    any_route_changed: bool,
}

impl RipDatabase {
    pub fn new() -> Self {
        return Self {
            ok_routes: HashMap::new(),
            any_route_changed: false,
        };
    }

    pub fn add_local_route(&mut self, entry: RipEntry, if_index: u32) -> RipResult<()> {
        let key = RipRouteKey {
            if_index,
            ip_address: entry.ip_address,
            subnet_mask: entry.subnet_mask,
            next_hop: entry.next_hop,
        };

        let value = RipDbEntry {
            rip_entry: entry,
            if_index,
            changed: false,
            is_local: true,
            in_routing_table: false,
            timeout_cnt: 0,
        };

        match self.ok_routes.entry(key) {
            Entry::Vacant(entry) => {
                println!("Adding local route: {}", key);
                entry.insert(value);
                Ok(())
            }
            Entry::Occupied(_) => Err(RipError::InvalidConfiguration(format!(
                "route already exists in RIP database: {}",
                key
            ))),
        }
    }

    pub fn get_routes_for_advertisement(
        &self,
        target_if_index: u32,
        changed_only: bool,
    ) -> impl Iterator<Item = RipEntry> + '_ {
        self.ok_routes
            .values()
            .filter(move |route| {
                if changed_only && !route.changed {
                    return false;
                }

                route.if_index != target_if_index
            })
            .map(|route| {
                let mut entry = route.rip_entry;
                entry.next_hop = 0;
                entry
            })
    }

    //pub fn update_from_rip(&mut self, entry: RipEntry, if_index: u32) -> bool;
    //pub fn mark_timeout_routes(&mut self);
    // pub fn collect_garbage(&mut self);
    // pub fn changed_routes(&self) -> impl Iterator<Item = &RipRouteDescription>;
    // pub fn all_routes(&self) -> impl Iterator<Item = &RipDbEntry> {
    //     self.ok_routes.values()
    // }
    // pub fn clear_changed_flags(&mut self);
    // pub fn any_route_changed(&self) -> bool;
}
