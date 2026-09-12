use crate::address::{is_net_mask_valid, is_unicast_address};
use crate::cfg::{AdvertisedNetwork, RipConfiguration};
use crate::result::{self, RIP_2_VERSION, RIP_UDP_PORT};
use crate::result::{RipError, RipResult};
use crate::rip_database::RipDatabase;
use crate::rip_ifc::RipIfc;
use crate::rip_packet::{RipEntry, RipIfInfo, RipPacket, RipPacketData};
use crate::rip_route::advertised_network_to_local_route;
use crate::rip_socket::RipSocket;
use crate::rip_updater::RipUpdater;
use crate::routing_table::{RoutingTable, RoutingTableDriver};
use std::env;
use std::future;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant, Sleep};

const RIP_INFINITY_METRIC: u32 = 16;
const RIP_REQUEST_WARMUP_MIN_MILLIS: u64 = 500;
const RIP_REQUEST_WARMUP_MAX_MILLIS: u64 = 1000;
const RIP_UPDATE_MIN_MILLIS: u64 = 27500;
const RIP_UPDATE_MAX_MILLIS: u64 = 35000;
const RIP_TRIGGERED_UPDATE_LOCK_MIN_MILLIS: u64 = 1000;
const RIP_TRIGGERED_UPDATE_LOCK_MAX_MILLIS: u64 = 5000;
const RIP_TIMEOUT_CHECK_INTERVAL_SECS: u64 = 15;
const RIP_ROUTE_TIMEOUT_SECS: u64 = 180;
const RIP_GARBAGE_COLLECTION_INTERVAL_SECS: u64 = 5;
const RIP_GARBAGE_COLLECTION_LIFETIME_SECS: u64 = 120;

pub struct RipDeamon<Driver>
where
    Driver: RoutingTableDriver,
{
    routing_table: RoutingTable<Driver>,
    interfaces: Vec<RipIfc>,
    database: RipDatabase,
    updater: RipUpdater,
}

async fn wait_optional_timer(timer: &mut Option<Pin<Box<Sleep>>>) {
    match timer {
        Some(timer) => timer.as_mut().await,
        None => future::pending::<()>().await,
    }
}

fn parse_rx_data(data: &[u8], source: SocketAddr, if_info: RipIfInfo) -> RipResult<RipPacket> {
    let SocketAddr::V4(source) = source else {
        return Err(RipError::InvalidSourceAddress());
    };

    let packet_data = RipPacketData::from_slice(&data)?;

    Ok(RipPacket {
        if_info,
        source,
        data: packet_data,
    })
}

fn spawn_rx_task(rx_socket: RipSocket, sender: mpsc::Sender<RipPacket>) {
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 2048];
        loop {
            let if_name = &rx_socket.if_name;
            let if_info = RipIfInfo {
                if_name: rx_socket.if_name.clone(),
                if_index: rx_socket.if_index,
            };
            match rx_socket.socket.recv_from(buffer.as_mut_slice()).await {
                Ok((len, source_addr)) => {
                    let data = &buffer[..len];
                    let packet = match parse_rx_data(data, source_addr, if_info) {
                        Ok(packet_data) => packet_data,
                        Err(err) => {
                            log::warn!("RIP receive error on {}: {}", if_name, err.to_string());
                            break;
                        }
                    };

                    let _ = sender.send(packet).await;
                }
                Err(error) => {
                    log::warn!("RIP receive error on {}: {}", if_name, error);
                    break;
                }
            }
        }
    });
}

fn is_metric_valid(metric: u32) -> bool {
    (1..=RIP_INFINITY_METRIC).contains(&metric)
}

fn is_entry_valid(entry: &RipEntry) -> bool {
    is_unicast_address(entry.ip_address)
        && is_net_mask_valid(entry.subnet_mask)
        && is_metric_valid(entry.metric)
}

fn update_metric(metric: u32) -> u32 {
    metric.saturating_add(1).min(RIP_INFINITY_METRIC)
}

fn format_entry(entry: &RipEntry) -> String {
    format!(
        "{}/{} via {} metric {}",
        Ipv4Addr::from(entry.ip_address),
        Ipv4Addr::from(entry.subnet_mask),
        Ipv4Addr::from(entry.next_hop),
        entry.metric
    )
}

fn random_millis_in_range(min_millis: u64, max_millis: u64) -> u64 {
    let range_len = max_millis - min_millis + 1;
    let random_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or_default();

    min_millis + random_seed % range_len
}

fn env_u64_or_default(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn create_request_warmup_duration() -> Duration {
    let warmup_millis =
        random_millis_in_range(RIP_REQUEST_WARMUP_MIN_MILLIS, RIP_REQUEST_WARMUP_MAX_MILLIS);

    Duration::from_millis(warmup_millis)
}

fn create_update_duration() -> Duration {
    let update_millis = random_millis_in_range(RIP_UPDATE_MIN_MILLIS, RIP_UPDATE_MAX_MILLIS);

    Duration::from_millis(update_millis)
}

fn create_triggered_update_lock_duration() -> Duration {
    let lock_millis = random_millis_in_range(
        RIP_TRIGGERED_UPDATE_LOCK_MIN_MILLIS,
        RIP_TRIGGERED_UPDATE_LOCK_MAX_MILLIS,
    );

    Duration::from_millis(lock_millis)
}

fn timeout_check_interval_secs() -> u64 {
    env_u64_or_default(
        "RIP_TIMEOUT_CHECK_INTERVAL_SECS",
        RIP_TIMEOUT_CHECK_INTERVAL_SECS,
    )
}

fn route_timeout_secs() -> u64 {
    env_u64_or_default("RIP_ROUTE_TIMEOUT_SECS", RIP_ROUTE_TIMEOUT_SECS)
}

fn garbage_collection_interval_secs() -> u64 {
    env_u64_or_default(
        "RIP_GARBAGE_COLLECTION_INTERVAL_SECS",
        RIP_GARBAGE_COLLECTION_INTERVAL_SECS,
    )
}

fn garbage_collection_lifetime_secs() -> u64 {
    env_u64_or_default(
        "RIP_GARBAGE_COLLECTION_LIFETIME_SECS",
        RIP_GARBAGE_COLLECTION_LIFETIME_SECS,
    )
}

fn response_entry_to_route(entry: &RipEntry, packet: &RipPacket) -> Option<RipEntry> {
    if !is_entry_valid(entry) {
        log::warn!("invalid RIP response entry: {:?}", entry);
        return None;
    }

    let mut route_entry = *entry;
    route_entry.metric = update_metric(route_entry.metric);
    route_entry.next_hop = u32::from(*packet.source.ip());

    Some(route_entry)
}

impl<Driver> RipDeamon<Driver>
where
    Driver: RoutingTableDriver,
{
    pub fn with_routing_table(routing_table: RoutingTable<Driver>) -> Self {
        return Self {
            routing_table,
            interfaces: vec![],
            database: RipDatabase::new(),
            updater: RipUpdater::new(),
        };
    }

    fn setup_sockets(&mut self, cfg: &RipConfiguration) -> RipResult<()> {
        for ifc_cfg in cfg.rip_interfaces.iter() {
            let if_name = ifc_cfg.dev.as_str();

            log::info!("configuring RIP interface {}", if_name);
            let rip_inteface = RipIfc::create(if_name)?;
            self.interfaces.push(rip_inteface);
        }

        Ok(())
    }

    fn setup_advertised_network(&mut self, network: &AdvertisedNetwork) -> RipResult<()> {
        let local_route = advertised_network_to_local_route(network)?;
        log::info!(
            "adding local advertised route {} on if_index {}",
            format_entry(&local_route.entry),
            local_route.if_index
        );
        self.database
            .add_local_route(local_route.entry, local_route.if_index)
    }

    pub fn setup(&mut self, cfg_path: &str) -> RipResult<()> {
        let rip_cfg = RipConfiguration::read_and_parse(cfg_path)?;
        self.setup_sockets(&rip_cfg)?;

        for advertised_network in rip_cfg.advertised_networks {
            self.setup_advertised_network(&advertised_network)?;
        }

        Ok(())
    }

    fn spawn_rx_tasks(&mut self, sender: mpsc::Sender<RipPacket>) {
        for ifc in self.interfaces.iter_mut() {
            if let Some(rx_socket) = ifc.rx.take() {
                spawn_rx_task(rx_socket, sender.clone());
            }
        }
    }

    pub async fn handle_packet(&mut self, packet: RipPacket) -> result::RipResult<()> {
        if packet.is_request() {
            log::info!(
                "handling RIP request from {} on {}",
                packet.source,
                packet.if_info.if_name
            );
            self.updater
                .rip_send_response_unicast(&self.database, &packet.source, &packet.if_info)
                .await
                .map_err(|err| result::RipError::IoError(err.to_string()))?;
        } else if packet.is_response() {
            self.handle_response_packet(&packet).await?;
        } else {
            return Err(RipError::MalformedPacket());
        }

        Ok(())
    }

    async fn handle_response_packet(&mut self, packet: &RipPacket) -> result::RipResult<()> {
        if packet.data.header.version != RIP_2_VERSION {
            return Err(RipError::MalformedPacket());
        }

        if packet.source.port() != RIP_UDP_PORT {
            log::warn!("ignoring RIP response from non-RIP port: {}", packet.source);
            return Ok(());
        }

        log::info!(
            "handling RIP response from {} on {} with {} entries",
            packet.source,
            packet.if_info.if_name,
            packet.data.entries.len()
        );

        for entry in packet.data.entries.iter() {
            let Some(route_entry) = response_entry_to_route(entry, packet) else {
                continue;
            };

            self.update_route_from_response(route_entry, packet.if_info.if_index)
                .await?;
        }

        Ok(())
    }

    async fn handle_poisoned_route_from_response(
        &mut self,
        route_entry: RipEntry,
        if_index: u32,
    ) -> result::RipResult<()> {
        log::info!(
            "received poisoned route {} on if_index {}",
            format_entry(&route_entry),
            if_index
        );

        let old_route = self.database.get_route(&route_entry, if_index).cloned();
        if let Some(old_route) = old_route {
            self.routing_table.delete_route(&old_route).await?;
            self.database.move_route_to_garbage(
                &old_route.rip_entry,
                old_route.if_index,
                std::time::Instant::now(),
            )?;
            return Ok(());
        }

        if self.database.has_garbage_route(&route_entry, if_index) {
            log::debug!(
                "poisoned route {} is already in garbage on if_index {}",
                format_entry(&route_entry),
                if_index
            );
            return Ok(());
        }

        log::debug!(
            "ignoring poisoned route for unknown route {} on if_index {}",
            format_entry(&route_entry),
            if_index
        );
        Ok(())
    }

    async fn update_route_from_response(
        &mut self,
        route_entry: RipEntry,
        if_index: u32,
    ) -> result::RipResult<()> {
        if route_entry.metric >= RIP_INFINITY_METRIC {
            return self
                .handle_poisoned_route_from_response(route_entry, if_index)
                .await;
        }

        let old_route = self.database.get_route(&route_entry, if_index).cloned();

        match old_route {
            None => {
                self.database.remove_garbage_route(&route_entry, if_index);
                let new_route = self.database.add_remote_route(route_entry, if_index)?;
                log::info!(
                    "learned new remote route {} on if_index {}",
                    format_entry(&route_entry),
                    if_index
                );
                self.routing_table.add_route(&new_route).await?;
            }
            Some(old_route) if old_route.rip_entry.metric > route_entry.metric => {
                log::info!(
                    "replacing route {} with better route {} on if_index {}",
                    format_entry(&old_route.rip_entry),
                    format_entry(&route_entry),
                    if_index
                );
                self.routing_table.delete_route(&old_route).await?;
                self.database
                    .remove_route(&old_route.rip_entry, old_route.if_index)?;

                let new_route = self.database.add_remote_route(route_entry, if_index)?;
                self.routing_table.add_route(&new_route).await?;
            }
            Some(old_route) => {
                self.database.refresh_route_timeout(&route_entry, if_index);
                log::debug!(
                    "keeping existing route {}, ignored candidate {}",
                    format_entry(&old_route.rip_entry),
                    format_entry(&route_entry)
                );
            }
        }

        Ok(())
    }

    async fn send_multicast_advertisement(&mut self, changed_only: bool) -> result::RipResult<()> {
        log::info!(
            "sending multicast advertisement, changed_only={}",
            changed_only
        );
        self.updater
            .rip_send_advertisement_multicast(&self.database, &self.interfaces, changed_only)
            .await
            .map_err(|err| result::RipError::IoError(err.to_string()))?;

        self.database.mark_all_routes_as_unchanged();

        Ok(())
    }

    async fn send_multicast_request(&self) -> result::RipResult<()> {
        log::info!(
            "sending multicast request on {} interfaces",
            self.interfaces.len()
        );
        for ifc in &self.interfaces {
            RipUpdater::rip_send_request_multicast(&ifc.tx)
                .await
                .map_err(|err| result::RipError::IoError(err.to_string()))?;
        }

        Ok(())
    }

    async fn maybe_send_triggered_update(
        &mut self,
        triggered_update_lock_timer: &mut Option<Pin<Box<Sleep>>>,
    ) -> result::RipResult<()> {
        if triggered_update_lock_timer.is_some() || !self.database.any_route_changed() {
            return Ok(());
        }

        let changed_only = true;
        log::info!("triggered update required");
        self.send_multicast_advertisement(changed_only).await?;
        *triggered_update_lock_timer = Some(Box::pin(time::sleep(
            create_triggered_update_lock_duration(),
        )));

        Ok(())
    }

    async fn handle_route_timeout_tick(
        &mut self,
        garbage_collection_timer: &mut Option<Pin<Box<Sleep>>>,
    ) -> result::RipResult<()> {
        let timeout_increment_secs = timeout_check_interval_secs();
        let timeout_limit_secs = route_timeout_secs();
        let timed_out_routes = self
            .database
            .collect_timed_out_routes(timeout_increment_secs, timeout_limit_secs);
        let should_start_garbage_collection =
            !timed_out_routes.is_empty() && garbage_collection_timer.is_none();

        for route in timed_out_routes {
            log::info!(
                "route timeout expired, moving to garbage: {} on if_index {}",
                format_entry(&route.rip_entry),
                route.if_index
            );
            self.routing_table.delete_route(&route).await?;
            self.database.move_route_to_garbage(
                &route.rip_entry,
                route.if_index,
                std::time::Instant::now(),
            )?;
        }

        if should_start_garbage_collection {
            log::debug!("starting garbage collection timer");
            *garbage_collection_timer = Some(Box::pin(time::sleep(Duration::from_secs(
                garbage_collection_interval_secs(),
            ))));
        }

        Ok(())
    }

    fn handle_garbage_collection_tick(
        &mut self,
        garbage_collection_timer: &mut Option<Pin<Box<Sleep>>>,
    ) {
        let garbage_lifetime = Duration::from_secs(garbage_collection_lifetime_secs());
        let removed_routes = self
            .database
            .remove_expired_garbage_routes(std::time::Instant::now(), garbage_lifetime);

        if !removed_routes.is_empty() {
            log::info!(
                "garbage collection removed {} RIP routes",
                removed_routes.len()
            );
        }

        if self.database.has_garbage_routes() {
            *garbage_collection_timer = Some(Box::pin(time::sleep(Duration::from_secs(
                garbage_collection_interval_secs(),
            ))));
        }
    }

    async fn handle_shutdown(&mut self) {
        log::info!("shutting down RIP daemon, advertising poisoned routes");
        let routes_to_delete = self.database.get_routes_in_routing_table();

        self.database.poison_all_routes();
        let changed_only = false;
        if let Err(err) = self.send_multicast_advertisement(changed_only).await {
            log::warn!(
                "failed to advertise poisoned routes during shutdown: {}",
                err.to_string()
            );
        }

        for route in routes_to_delete {
            log::info!(
                "deleting route from kernel during shutdown: {} on if_index {}",
                format_entry(&route.rip_entry),
                route.if_index
            );
            if let Err(err) = self.routing_table.delete_route(&route).await {
                log::warn!(
                    "failed to delete route during shutdown: {}, route {} on if_index {}",
                    err.to_string(),
                    format_entry(&route.rip_entry),
                    route.if_index
                );
            }
        }
    }

    pub async fn run(&mut self) -> result::RipResult<()> {
        let mut request_warmup_timer =
            Some(Box::pin(time::sleep(create_request_warmup_duration())));
        let mut update_timer = Box::pin(time::sleep(create_update_duration()));
        let mut triggered_update_lock_timer: Option<Pin<Box<Sleep>>> = None;
        let mut timeout_timer = Box::pin(time::sleep(Duration::from_secs(
            timeout_check_interval_secs(),
        )));
        let mut garbage_collection_timer: Option<Pin<Box<Sleep>>> = None;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<RipPacket>(64);
        self.spawn_rx_tasks(tx);
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|err| result::RipError::IoError(err.to_string()))?;

        loop {
            let event_result: result::RipResult<()> = tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    result.map_err(|err| result::RipError::IoError(err.to_string()))?;
                    self.handle_shutdown().await;
                    return Ok(());
                }

                _ = sigterm.recv() => {
                    self.handle_shutdown().await;
                    return Ok(());
                }

                _ = wait_optional_timer(&mut request_warmup_timer) => {
                    request_warmup_timer = None;

                    log::info!("request warmup timer triggered");
                    self.send_multicast_request().await?;

                    Ok(())
                }

                _ = &mut update_timer => {
                    let changed_only = false;
                    self.send_multicast_advertisement(changed_only).await?;

                    update_timer
                        .as_mut()
                        .reset(Instant::now() + create_update_duration());

                    Ok(())
                }

                _ = wait_optional_timer(&mut triggered_update_lock_timer) => {
                    triggered_update_lock_timer = None;

                    Ok(())
                }

                _ = &mut timeout_timer => {
                    self.handle_route_timeout_tick(&mut garbage_collection_timer).await?;

                    timeout_timer
                        .as_mut()
                        .reset(Instant::now() + Duration::from_secs(timeout_check_interval_secs()));

                    Ok(())
                }

                _ = wait_optional_timer(&mut garbage_collection_timer) => {
                    garbage_collection_timer = None;
                    self.handle_garbage_collection_tick(&mut garbage_collection_timer);

                    Ok(())
                }

                received = rx.recv() => {
                    let Some(packet) = received else {
                        return Err(result::RipError::IoError(
                            "all RIP receive tasks stopped".to_string()
                        ));
                    };

                    log::info!(
                        "received packet from {} on {}, with entries count {}",
                        packet.source,
                        packet.if_info.if_name,
                        packet.data.entries.len()
                    );

                    self.handle_packet(packet).await?;

                    Ok(())
                }

            };

            event_result?;
            self.maybe_send_triggered_update(&mut triggered_update_lock_timer)
                .await?;
        }
    }
}

#[cfg(test)]
#[path = "tests/rip_deamon_tests.rs"]
mod tests;
