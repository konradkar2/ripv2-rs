use std::fs;

use crate::common::{Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RipInterface {
   pub dev: String,
}

#[derive(Debug, Deserialize)]
pub struct AdvertisedNetwork {
   pub address: String,
   pub prefix: u32,
   pub dev: String
}

#[derive(Debug, Deserialize)]
pub struct RipConfiguration {
  pub version: u32,
  pub rip_interfaces: Vec<RipInterface>,
  pub advertised_networks: Vec<AdvertisedNetwork>
}

#[derive(Debug, Deserialize)]
pub struct Cfg {
    rip_configuration: RipConfiguration,
}


impl RipConfiguration {
    pub fn parse(content: &str) -> Result<RipConfiguration> {
        let config: std::result::Result<Cfg, _> = serde_saphyr::from_str(content);
        let config = config.map_err(|err| { Error::InvalidConfiguration(err.to_string()) })?;

        Ok(config.rip_configuration)
    }

    pub fn read_and_parse(path: &str) -> Result<RipConfiguration> {
        let contents = fs::read_to_string(path).map_err(|err| {
            return Error::InvalidConfiguration(format!("{}: {}", err.to_string(), path));
        })?;

        RipConfiguration::parse(contents.as_str())
    }
}





