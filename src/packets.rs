use protocol::Parcel;
use protocol_derive::Protocol;

pub type Filter = Box<[u8; 65536]>;  // TODO change 65536 to a const generic

#[derive(Protocol, Debug, Clone)]
pub enum Packet {
    UpdateFilters(UpdateFiltersPacket),
    Dummy(DummyPacket),
}

#[derive(Protocol, Debug, Clone)]
pub struct UpdateFiltersPacket {
    filters: Vec<Filter>,
}

#[derive(Protocol, Debug, Clone)]
pub struct DummyPacket {
    foo: usize,
}
