use tokio::time::{self, Duration, Instant};
use crate::routing_table::RoutingTable;

pub struct RipDeamon {
    routing_table: RoutingTable,
}

impl RipDeamon {
    pub fn new() -> Self {
        return Self {
            routing_table: RoutingTable::new()
        }
    }

    pub async fn run(self: &Self) {
        let route_timeout_1 = time::sleep(Duration::from_secs(5));
        tokio::pin!(route_timeout_1);
        loop {
            tokio::select! {
                 _ = &mut route_timeout_1 => {
                     println!("RipDeamon timer triggered");
                    
                    route_timeout_1
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(5));
                }
            }
        }
    }
}