use crate::common::{Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RipInterface {
   dev: String,
}

#[derive(Debug, Deserialize)]
pub struct AdvertisedNetwork {
   address: String,
   prefix: u32,
   dev: String
}

#[derive(Debug, Deserialize)]
pub struct RipConfiguration {
  version: u32,
  rip_interfaces: Vec<RipInterface>,
  advertised_networks: Vec<AdvertisedNetwork>
}

#[derive(Debug, Deserialize)]
pub struct Cfg {
    rip_configuration: RipConfiguration,
}

pub fn parse(content: &str) -> Result<RipConfiguration> {
    let config: std::result::Result<Cfg, _> = serde_saphyr::from_str(content);
    let config = config.map_err(|err| { Error::InvalidConfiguration(err.to_string()) })?;

    Ok(config.rip_configuration)

}
