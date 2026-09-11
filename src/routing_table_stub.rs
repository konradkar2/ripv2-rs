use crate::result::RipResult;
use crate::rip_database::RipDbEntry;
use crate::routing_table::RoutingTableDriver;

#[derive(Debug, Default)]
pub struct StubRoutingTableDriver {
    pub added_routes: Vec<RipDbEntry>,
    pub deleted_routes: Vec<RipDbEntry>,
}

impl StubRoutingTableDriver {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RoutingTableDriver for StubRoutingTableDriver {
    async fn add_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        self.added_routes.push(route.clone());
        Ok(())
    }

    async fn delete_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        self.deleted_routes.push(route.clone());
        Ok(())
    }
}
