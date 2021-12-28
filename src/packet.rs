use crate::protocol::ProtocolVersion;

pub trait Packet {
    fn read() -> Self;
    fn write(&self, buf: &mut [u8]) -> Option<usize>;
    fn get_id(pv: ProtocolVersion) -> i32 {
        pv.get_id::<Self>()
    }
    // fn get_kind() -> PacketKind;
}
