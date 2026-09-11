use crate::result::RipResult;
use crate::rip_database::RipDbEntry;

pub struct RoutingTable {}

impl RoutingTable {
    pub fn new() -> Self {
        return Self {};
    }

    pub fn add_route(&mut self, _route: &RipDbEntry) -> RipResult<()> {
        Ok(())
    }

    pub fn delete_route(&mut self, _route: &RipDbEntry) -> RipResult<()> {
        Ok(())
    }
}
