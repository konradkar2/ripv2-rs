use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use crate::result::{RipError, RipResult};
use crate::rip_packet::RipEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RipRouteKey {
    pub ip_address: u32,
    pub subnet_mask: u32,
}

impl fmt::Display for RipRouteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ip_address={}, subnet_mask={}",
            Ipv4Addr::from(self.ip_address),
            Ipv4Addr::from(self.subnet_mask),
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
    pub timeout_cnt: u64,
    pub garbage_started_at: Option<Instant>,
}

pub struct RipDatabase {
    pub ok_routes: HashMap<RipRouteKey, RipDbEntry>,
    pub garbage_routes: HashMap<RipRouteKey, RipDbEntry>,
    any_route_changed: bool,
}

impl RipDatabase {
    pub fn new() -> Self {
        return Self {
            ok_routes: HashMap::new(),
            garbage_routes: HashMap::new(),
            any_route_changed: false,
        };
    }

    pub fn add_local_route(&mut self, entry: RipEntry, if_index: u32) -> RipResult<()> {
        let changed = true;
        let is_local = true;
        let in_routing_table = false;

        self.add_route(entry, if_index, changed, is_local, in_routing_table)
            .map(|_| ())
    }

    pub fn get_route(&self, entry: &RipEntry) -> Option<&RipDbEntry> {
        let key = Self::build_route_key(entry);
        self.ok_routes.get(&key)
    }

    pub fn add_remote_route(&mut self, entry: RipEntry, if_index: u32) -> RipResult<RipDbEntry> {
        let changed = true;
        let is_local = false;
        let in_routing_table = true;

        self.add_route(entry, if_index, changed, is_local, in_routing_table)
    }

    fn add_route(
        &mut self,
        entry: RipEntry,
        if_index: u32,
        changed: bool,
        is_local: bool,
        in_routing_table: bool,
    ) -> RipResult<RipDbEntry> {
        let key = Self::build_route_key(&entry);

        let value = RipDbEntry {
            rip_entry: entry,
            if_index,
            changed,
            is_local,
            in_routing_table,
            timeout_cnt: 0,
            garbage_started_at: None,
        };

        match self.ok_routes.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value.clone());
                if value.changed {
                    self.any_route_changed = true;
                }
                Ok(value)
            }
            Entry::Occupied(_) => Err(RipError::InvalidConfiguration(format!(
                "route already exists in RIP database: {}",
                key
            ))),
        }
    }

    pub fn remove_route(&mut self, entry: &RipEntry) -> RipResult<RipDbEntry> {
        let key = Self::build_route_key(entry);

        if let Some(route) = self.ok_routes.remove(&key) {
            return Ok(route);
        }

        self.garbage_routes
            .remove(&key)
            .ok_or(RipError::InvalidConfiguration(format!(
                "route does not exist in RIP database: {}",
                key
            )))
    }

    pub fn remove_garbage_route(&mut self, entry: &RipEntry) -> Option<RipDbEntry> {
        let key = Self::build_route_key(entry);
        self.garbage_routes.remove(&key)
    }

    pub fn has_garbage_route(&self, entry: &RipEntry) -> bool {
        let key = Self::build_route_key(entry);
        self.garbage_routes.contains_key(&key)
    }

    pub fn refresh_route_timeout(&mut self, entry: &RipEntry) {
        let key = Self::build_route_key(entry);
        if let Some(route) = self.ok_routes.get_mut(&key) {
            route.timeout_cnt = 0;
        }
    }

    pub fn collect_timed_out_routes(
        &mut self,
        timeout_increment_secs: u64,
        timeout_limit_secs: u64,
    ) -> Vec<RipDbEntry> {
        self.ok_routes
            .values_mut()
            .filter_map(|route| {
                if route.is_local {
                    return None;
                }

                route.timeout_cnt = route.timeout_cnt.saturating_add(timeout_increment_secs);
                if route.timeout_cnt >= timeout_limit_secs {
                    return Some(route.clone());
                }

                None
            })
            .collect()
    }

    pub fn move_route_to_garbage(
        &mut self,
        entry: &RipEntry,
        garbage_started_at: Instant,
    ) -> RipResult<RipDbEntry> {
        let key = Self::build_route_key(entry);
        let mut route = self
            .ok_routes
            .remove(&key)
            .ok_or(RipError::InvalidConfiguration(format!(
                "route does not exist in RIP database: {}",
                key
            )))?;

        route.changed = true;
        route.rip_entry.metric = 16;
        route.in_routing_table = false;
        route.garbage_started_at = Some(garbage_started_at);
        self.any_route_changed = true;

        match self.garbage_routes.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(route.clone());
                Ok(route)
            }
            Entry::Occupied(_) => Err(RipError::InvalidConfiguration(format!(
                "route already exists in RIP garbage database: {}",
                key
            ))),
        }
    }

    pub fn remove_expired_garbage_routes(
        &mut self,
        now: Instant,
        garbage_lifetime: Duration,
    ) -> Vec<RipDbEntry> {
        let expired_keys: Vec<RipRouteKey> = self
            .garbage_routes
            .iter()
            .filter_map(|(key, route)| {
                let garbage_started_at = route.garbage_started_at?;
                if now.duration_since(garbage_started_at) >= garbage_lifetime {
                    return Some(*key);
                }

                None
            })
            .collect();

        expired_keys
            .into_iter()
            .filter_map(|key| self.garbage_routes.remove(&key))
            .collect()
    }

    pub fn has_garbage_routes(&self) -> bool {
        !self.garbage_routes.is_empty()
    }

    pub fn poison_all_routes(&mut self) {
        if self.ok_routes.is_empty() && self.garbage_routes.is_empty() {
            return;
        }

        for route in self
            .ok_routes
            .values_mut()
            .chain(self.garbage_routes.values_mut())
        {
            route.rip_entry.metric = 16;
            route.changed = true;
        }

        self.any_route_changed = true;
    }

    pub fn get_routes_in_routing_table(&self) -> Vec<RipDbEntry> {
        self.ok_routes
            .values()
            .chain(self.garbage_routes.values())
            .filter(|route| route.in_routing_table)
            .cloned()
            .collect()
    }

    pub fn get_routes_for_advertisement(
        &self,
        target_if_index: u32,
        changed_only: bool,
    ) -> impl Iterator<Item = RipEntry> + '_ {
        self.ok_routes
            .values()
            .chain(self.garbage_routes.values())
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

    pub fn any_route_changed(&self) -> bool {
        self.any_route_changed
    }

    pub fn mark_all_routes_as_unchanged(&mut self) {
        if !self.any_route_changed {
            return;
        }

        for route in self.ok_routes.values_mut() {
            route.changed = false;
        }
        for route in self.garbage_routes.values_mut() {
            route.changed = false;
        }

        self.any_route_changed = false;
    }

    fn build_route_key(entry: &RipEntry) -> RipRouteKey {
        RipRouteKey {
            ip_address: entry.ip_address,
            subnet_mask: entry.subnet_mask,
        }
    }
}

#[cfg(test)]
#[path = "tests/rip_database_tests.rs"]
mod tests;
