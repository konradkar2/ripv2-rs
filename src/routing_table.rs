use crate::result::RipResult;
use crate::rip_database::RipDbEntry;

#[allow(async_fn_in_trait)]
pub trait RoutingTableDriver {
    async fn add_route(&mut self, route: &RipDbEntry) -> RipResult<()>;
    async fn delete_route(&mut self, route: &RipDbEntry) -> RipResult<()>;
}

pub struct RoutingTable<Driver>
where
    Driver: RoutingTableDriver,
{
    driver: Driver,
}

impl<Driver> RoutingTable<Driver>
where
    Driver: RoutingTableDriver,
{
    pub fn with_driver(driver: Driver) -> Self {
        Self { driver }
    }

    pub async fn add_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        self.driver.add_route(route).await
    }

    pub async fn delete_route(&mut self, route: &RipDbEntry) -> RipResult<()> {
        self.driver.delete_route(route).await
    }

    #[cfg(test)]
    pub fn driver(&self) -> &Driver {
        &self.driver
    }
}
