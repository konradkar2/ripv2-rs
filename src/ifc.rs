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

impl RipEntry {
    pub fn to_be(&mut self) {
        self.routing_family_id = self.routing_family_id.to_be();
        self.route_tag = self.route_tag.to_be();
        self.ip_address = self.ip_address.to_be();
        self.subnet_mask = self.subnet_mask.to_be();
        self.next_hop = self.next_hop.to_be();
        self.metric = self.metric.to_be();
    }

    pub fn to_le(&mut self) {
        self.routing_family_id = self.routing_family_id.to_le();
        self.route_tag = self.route_tag.to_le();
        self.ip_address = self.ip_address.to_le();
        self.subnet_mask = self.subnet_mask.to_le();
        self.next_hop = self.next_hop.to_le();
        self.metric = self.metric.to_le();
    }
}

pub const RIP_ENTRY_SIZE: usize = 20;
 
const _: [(); std::mem::size_of::<RipHeader>()] = [(); RIP_HEADER_SIZE];
const _: [(); std::mem::size_of::<RipEntry>()] = [(); RIP_ENTRY_SIZE];