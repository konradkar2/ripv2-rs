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

#[cfg(test)]
mod tests {
    use super::prefix_to_mask;

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
}
