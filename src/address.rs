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
mod tests {
    use super::{is_net_mask_valid, is_unicast_address, prefix_to_mask};

    #[test]
    fn converts_prefix_to_mask() {
        assert_eq!(prefix_to_mask(0).unwrap(), 0x00000000);
        assert_eq!(prefix_to_mask(24).unwrap(), 0xffffff00);
        assert_eq!(prefix_to_mask(32).unwrap(), 0xffffffff);
    }

    #[test]
    fn rejects_invalid_prefix() {
        assert!(prefix_to_mask(33).is_err());
    }

    #[test]
    fn validates_net_masks() {
        assert!(is_net_mask_valid(0xffffff00));
        assert!(is_net_mask_valid(0xfffffffe));
        assert!(!is_net_mask_valid(0xffffffff));
        assert!(!is_net_mask_valid(0x00000000));
        assert!(!is_net_mask_valid(0x80ffff00));
        assert!(!is_net_mask_valid(0xfeffff00));
    }

    #[test]
    fn validates_unicast_addresses() {
        assert!(is_unicast_address(u32::from(std::net::Ipv4Addr::new(
            10, 0, 0, 1
        ))));
        assert!(!is_unicast_address(u32::from(std::net::Ipv4Addr::new(
            0, 0, 0, 1
        ))));
        assert!(!is_unicast_address(u32::from(std::net::Ipv4Addr::new(
            127, 0, 0, 1
        ))));
        assert!(!is_unicast_address(u32::from(std::net::Ipv4Addr::new(
            224, 0, 0, 1
        ))));
    }
}
