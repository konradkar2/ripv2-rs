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
