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
    /// Asks to apply our query on its documents and return results in the [ResponsePacket::ReturnResults] packet.
    Search(SearchPacket),

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
pub struct SearchPacket {
    query: String,
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
    /// Response to a [RequestPacket::Search] packet.
    Results(ResultsPacket),

    Disconnect(DisconnectPacket),
}

#[derive(Protocol, Debug, Clone)]
pub struct UpdateFiltersPacket {
    /// The filters ordered from distance 0 to the furthest at a distance of [RefreshPacket::range].
    pub filters: Vec<Filter>,
}

#[derive(Protocol, Debug, Clone)]
pub struct ResultsPacket {
    peers: Vec<(PeerId, Vec<u64>)>,
    results: Vec<String>,
}

#[derive(Protocol, Debug, Clone)]
pub struct DisconnectPacket {
    /// The reason for the disconnection.
    pub reason: String,
    /// Asks the peer to reconnect after a certain amount of time.
    /// None if we never want to hear about that peer again.
    pub try_again_in: Option<u32>,
}
