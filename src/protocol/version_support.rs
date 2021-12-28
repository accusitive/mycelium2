
use crate::packet::Packet;

use super::{Mc1_12, ProtocolVersion};

impl Mc1_12 {
    pub fn get_id<P: Packet + ?Sized>(&self) -> i32 {
        
        match self {
            _ => 0,
        }
    }
}
impl ProtocolVersion {
    pub fn get_id<P: Packet + ?Sized>(&self) -> i32 {
        match self {
            ProtocolVersion::Mc1_12(mc) => mc.get_id::<P>(),
        }
    }
}
