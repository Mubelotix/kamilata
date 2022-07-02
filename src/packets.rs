use protocol::Parcel;
use protocol_derive::Protocol;

pub type Filter = Vec<u8>;  // TODO change to Box<[u8; N]>

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
    foo: u64,
}
