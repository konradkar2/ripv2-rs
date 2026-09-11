use crate::result::{RipError, RipResult};

pub fn prefix_to_mask(prefix: u32) -> RipResult<u32> {
    if prefix > 32 {
        return Err(RipError::InvalidConfiguration(format!(
            "invalid prefix: {}",
            prefix
        )));
    }

    if prefix == 0 {
        return Ok(0);
    }

    Ok(u32::MAX << (32 - prefix))
}

pub fn is_unicast_address(address: u32) -> bool {
    let msb = address >> 24;

    if msb == 0 {
        return false;
    }

    if msb == 127 {
        return false;
    }

    if msb >= 224 {
        return false;
    }

    true
}

pub fn is_net_mask_valid(net_mask: u32) -> bool {
    if net_mask == 0 || net_mask == u32::MAX {
        return false;
    }

    net_mask.trailing_zeros() + (!net_mask).leading_zeros() == 32
}

#[cfg(test)]
#[path = "tests/address_tests.rs"]
mod tests;
