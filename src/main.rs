use tokio::time::{self, Duration, Instant};
mod rip_deamon;
use rip_deamon::RipDeamon;
mod routing_table;
mod rip_socket;

async fn run_rip_deamon() {
    let deamon = RipDeamon::new();
    deamon.run().await

}

#[tokio::main]
async fn main() {

    tokio::spawn(async {
        run_rip_deamon().await;
    });

    let route_timeout_1 = time::sleep(Duration::from_secs(5));
    tokio::pin!(route_timeout_1);

    let route_timeout_2 = time::sleep(Duration::from_secs(20));
    tokio::pin!(route_timeout_2);

    let route_timeout_3 = time::sleep(Duration::from_secs(60));
    tokio::pin!(route_timeout_3);

    loop {
        tokio::select! {
            _ = &mut route_timeout_1 => {
                println!("timeout_1 expire");

                route_timeout_1
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(5));
            }

            _ = &mut route_timeout_2 => {
                println!("timeout_2 expire");

                route_timeout_2
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(20));
            }

            _ = &mut route_timeout_3 => {
                println!("timeout_3 expire");

                route_timeout_3
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(60));
            }
        }
    }
}
