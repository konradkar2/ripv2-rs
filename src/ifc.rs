use std::mem::size_of;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RipHeader {
    pub command: u8,
    pub version: u8,
    pub padding: u16,
}
pub const RIP_HEADER_SIZE: usize = 4;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RipEntry {
    pub routing_family_id: u16,
    pub route_tag: u16,
    pub ip_address: u32,
    pub subnet_mask: u32,
    pub next_hop: u32,
    pub metric: u32,
}

pub const RIP_ENTRY_SIZE: usize = 20;
 
const _: [(); std::mem::size_of::<RipHeader>()] = [(); RIP_HEADER_SIZE];
const _: [(); std::mem::size_of::<RipEntry>()] = [(); RIP_ENTRY_SIZE];