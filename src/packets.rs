use protocol::Parcel;
use protocol_derive::Protocol;

pub type Filter = Vec<u8>;  // TODO change to Box<[u8; N]>

#[derive(Protocol, Debug, Clone)]
pub enum RequestPacket {

}

#[derive(Protocol, Debug, Clone)]
pub enum ResponsePacket {

}
