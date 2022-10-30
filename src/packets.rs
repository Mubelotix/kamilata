use crate::prelude::*;
use protocol::Parcel;
use protocol_derive::Protocol;

// TODO everything pub crate

type PeerId = Vec<u8>;  // TODO change to libp2p PeerId
type Filter = Vec<u8>;

#[derive(Protocol, Debug, Clone)]
pub enum RequestPacket {
    /// Nodes automatically notify their peers of their filter changes using the [ResponsePacket::UpdateFilters] packet.
    /// The [RequestPacket::SetRefresh] packet is used to specify what information we want to receive from our peers (and how often).
    /// This packet also marks the used substream as the substream on which all the updates will be sent.
    /// This receiver will instantly reply to this request using a [ResponsePacket::ConfirmRefresh] packet.
    SetRefresh(RefreshPacket),
    /// Asks for contact information of peers matching our request.
    /// The receiver will reply with a [ResponsePacket::ReturnPeers] packet.
    FindPeers(FindPeersPacket),
    /// Asks to apply our query on its documents and return results in the [ResponsePacket::ReturnResults] packet.
    Search(SearchPacket),
    /// Gives a hint about the usefulness of a peer.
    /// This can be used to compute reputation of a peer.
    RewardPeer(RewardPeerPacket),

    Disconnect(DisconnectPacket),
}

#[derive(Protocol, Debug, Clone)]
pub struct RefreshPacket {
    /// Which filters we want to receive.
    /// The farthest filter will be at a distance of `range`.
    /// The closest filter will always be 0 so the number of filters will be `range + 1`.
    pub range: u8,
    /// Milliseconds between each update.
    /// This does not force packets to be sent as they will still wait for the filters to change before updating them.
    pub interval: u64,
    /// Peers we don't want to hear from because we think they are malicious.
    pub blocked_peers: Vec<PeerId>,
}

impl Default for RefreshPacket {
    fn default() -> Self {
        RefreshPacket {
            range: 6,
            interval: 21 * 1000,
            blocked_peers: Vec::new(),
        }
    }
}

#[derive(Protocol, Debug, Clone)]
pub struct FindPeersPacket {
    /// The maximum distance at which the query can be found from that peer.
    range: u8,
    query: Vec<Vec<u32>>,
}

#[derive(Protocol, Debug, Clone)]
pub struct SearchPacket {
    query: Vec<Vec<String>>,
}

#[derive(Protocol, Debug, Clone)]
pub struct RewardPeerPacket {
    /// The peer id of the peer we want to reward.
    peer_id: PeerId,
    /// Some value between -1 and 1 that represents our evaluation of the usefulness of the peer.
    /// It is adivised to add malicious peers to [RefreshPacket::blocked_peers] to prevent them from causing future harm.
    reward: f32,
    // TODO: Maybe add a PoW field to prevent spam of malicious reward packets
}

#[derive(Protocol, Debug, Clone)]
pub enum ResponsePacket {
    /// Response to a [RequestPacket::SetRefresh] packet.
    /// The inner packets can differ if the demanded settings are deemed unacceptable by the responder.
    /// The responder has the final say on the settings.
    /// The requester should disconnect if the peers cannot agree.
    ConfirmRefresh(RefreshPacket),
    /// Sent periodically to inform the peers of our filters.
    UpdateFilters(UpdateFiltersPacket),
    /// Response to a [RequestPacket::FindPeers] packet.
    ReturnPeers(ReturnPeersPacket),
    /// Response to a [RequestPacket::Search] packet.
    ReturnResults(ReturnResultsPacket),

    Disconnect(DisconnectPacket),
}

#[derive(Protocol, Debug, Clone)]
pub struct UpdateFiltersPacket {
    /// The filters ordered from distance 0 to the furthest at a distance of [RefreshPacket::range].
    pub filters: Vec<Filter>,
}

#[derive(Protocol, Debug, Clone)]
pub struct ReturnPeersPacket {
    /// An array of levels.
    /// At each level is an array of peers.
    /// The index of the level is the distance between the searched query and the returned peer.
    /// The number of levels is defined by the [FindPeersPacket::range] field.
    /// 
    /// Note: First item is at a distance of 0, meaning it can at most contain one peer: the sender.
    peers: Vec<Vec<PeerId>>,
}

#[derive(Protocol, Debug, Clone)]
pub struct ReturnResultsPacket {
    /// Return results, associated with a score.
    /// The score should be calculated by an algorithm that must be common to all peers, so that scores can be checked locally.
    results: Vec<(Vec<u8>, f64)>,
    // TODO: signature so that we can report them
}

#[derive(Protocol, Debug, Clone)]
pub struct DisconnectPacket {
    /// The reason for the disconnection.
    pub reason: String,
    /// Asks the peer to reconnect after a certain amount of time.
    /// None if we never want to hear about that peer again.
    pub try_again_in: Option<u32>,
}
