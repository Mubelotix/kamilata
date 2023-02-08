use protocol_derive::Protocol;

#[derive(Clone)]
#[repr(transparent)]
pub struct PeerId(libp2p::PeerId);

impl From<libp2p::PeerId> for PeerId {
    fn from(peer_id: libp2p::PeerId) -> Self {
        PeerId(peer_id)
    }
}

impl From<PeerId> for libp2p::PeerId {
    fn from(peer_id: PeerId) -> Self {
        peer_id.0
    }
}

impl protocol::Parcel for PeerId {
    const TYPE_NAME: &'static str = "PeerId";

    fn read_field(read: &mut dyn std::io::Read,
                  settings: &protocol::Settings,
                  hints: &mut protocol::hint::Hints) -> Result<Self, protocol::Error> {
        let lenght: u16 = protocol::Parcel::read_field(read, settings, hints)?;
        let mut bytes = vec![0; lenght as usize];
        read.read_exact(&mut bytes)?;
        Ok(PeerId(libp2p::PeerId::from_bytes(&bytes).unwrap()))
    }

    fn write_field(&self, write: &mut dyn std::io::Write,
             settings: &protocol::Settings,
             hints: &mut protocol::hint::Hints) -> Result<(), protocol::Error> {
        let bytes = self.0.to_bytes();
        let lenght = bytes.len() as u16;
        lenght.write_field(write, settings, hints)?;
        write.write_all(&bytes)?;
        Ok(())
    }
}

impl std::fmt::Debug for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub trait HackTraitVecPeerId {
    fn to_libp2p_peer_ids(self) -> Vec<libp2p::PeerId>;
}
impl HackTraitVecPeerId for Vec<PeerId> {
    fn to_libp2p_peer_ids(self) -> Vec<libp2p::PeerId> {
        unsafe {
            std::mem::transmute_copy(&self)
        }
    }
}

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

/// For a filter to match a query, it must have at least `match_count` bits set to 1 at the positions specified by hashed `words`.
#[derive(Protocol, Debug, Clone)]
pub struct Query {
    /// List of words to search for.
    pub words: Vec<String>,
    /// Minimum number of words that must match in order for a filter to match the query.
    /// Invalid if greater than `words.len()`.
    pub min_matching: u16,
}

#[derive(Protocol, Debug, Clone)]
pub struct SearchPacket {
    /// A list of queries.
    /// When multiple queries are sent, all of them should be replied to.
    /// If not possible, the responder should reply with as many as possible, starting with the first.
    pub queries: Vec<Query>,
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
    pub filters: Vec<Vec<u8>>,
}

#[derive(Protocol, Debug, Clone)]
pub struct DistantMatch {
    /// The first (thus best) query this result matched at a distance of the corresponding index.
    /// At least one of the items in this list should be `Some`.
    pub queries: Vec<Option<u16>>,
    pub peer_id: PeerId,
    pub addresses: Vec<String>,
}

#[derive(Protocol, Debug, Clone)]
pub struct LocalMatch {
    /// The first (thus best) query this result matched.
    pub query: u16,
    /// The result to be deserialized and used.
    pub result: Vec<u8>,
}

#[derive(Protocol, Debug, Clone)]
pub struct ResultsPacket {
    /// A list of routing information to be used to find actual results.
    pub distant_matches: Vec<DistantMatch>,
    /// Contains a list of [SearchResult]s to be deserialized and used.
    pub matches: Vec<LocalMatch>,
}

#[derive(Protocol, Debug, Clone)]
pub struct DisconnectPacket {
    /// The reason for the disconnection.
    pub reason: String,
    /// Asks the peer to reconnect after a certain amount of time.
    /// None if we never want to hear about that peer again.
    pub try_again_in: Option<u32>,
}
