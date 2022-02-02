use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::mem::size_of;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::RangeBounds;
use std::sync::Arc;

use async_channel::Sender;
use tokio::sync::Mutex;

use executor::{Handler, ModuleRef, System};

/// An identifier of a node in Chord.
pub(crate) type ChordId = u128;

/// Returns the minimal Chord identifier value
/// for a given number of bits.
pub(crate) fn chord_id_min(_ring_bits: usize) -> ChordId {
    0
}

/// Returns the maximal Chord identifier value
/// for a given number of bits.
pub(crate) fn chord_id_max(ring_bits: usize) -> ChordId {
    !(&(ChordId::MAX).checked_shl(ring_bits as u32).unwrap_or(0))
}

/// Returns a given chord identifier incremented
/// by a given delta clockwise in the identifier
/// (ring) space with a given number of bits.
pub(crate) fn chord_id_advance_by(ring_bits: usize, base: &ChordId, delta: &ChordId) -> ChordId {
    base.wrapping_add(*delta) & chord_id_max(ring_bits)
}

/// Computes the distance between two Chord
/// identifiers in the clockwise direction in
/// the identifier (ring) space with a given
/// number of bits.
pub(crate) fn chord_id_distance(ring_bits: usize, from: &ChordId, to: &ChordId) -> ChordId {
    if to >= from {
        to - from
    } else {
        (chord_id_max(ring_bits) - from) + (to - chord_id_min(ring_bits)) + 1
    }
}

/// Checks if a given identifier falls within
/// a given range of Chord identifiers, where
/// the range is interpreted clockwise in the
/// identifier (ring) space with a given
/// number of bits.
pub(crate) fn chord_id_in_range<R>(ring_bits: usize, id: &ChordId, range: R) -> bool
where
    R: RangeBounds<ChordId>,
{
    match range.start_bound() {
        Included(sb) => match range.end_bound() {
            Included(eb) => match sb.cmp(eb) {
                Ordering::Equal => id == sb,
                Ordering::Less => id >= sb && id <= eb,
                Ordering::Greater => {
                    (id >= sb && id <= &chord_id_max(ring_bits))
                        || (id >= &chord_id_min(ring_bits) && id <= eb)
                }
            },
            Excluded(eb) => match sb.cmp(eb) {
                Ordering::Equal => false,
                Ordering::Less => id >= sb && id < eb,
                Ordering::Greater => {
                    (id >= sb && id <= &chord_id_max(ring_bits))
                        || (id >= &chord_id_min(ring_bits) && id < eb)
                }
            },
            Unbounded => panic!("Unbounded range disallowed!"),
        },
        Excluded(sb) => match range.end_bound() {
            Included(eb) => match sb.cmp(eb) {
                Ordering::Equal => true,
                Ordering::Less => id > sb && id <= eb,
                Ordering::Greater => {
                    (id > sb && id <= &chord_id_max(ring_bits))
                        || (id >= &chord_id_min(ring_bits) && id <= eb)
                }
            },
            Excluded(eb) => match sb.cmp(eb) {
                Ordering::Equal => panic!("Empty range disallowed!"),
                Ordering::Less => id > sb && id < eb,
                Ordering::Greater => {
                    (id > sb && id <= &chord_id_max(ring_bits))
                        || (id >= &chord_id_min(ring_bits) && id < eb)
                }
            },
            Unbounded => panic!("Unbounded range disallowed!"),
        },
        Unbounded => panic!("Unbounded range disallowed!"),
    }
}

/// The maximal number of entries in
/// a Chord finger table.
pub(crate) const CHORD_FINGER_TABLE_MAX_ENTRIES: usize = size_of::<ChordId>() << 3;

/// The maximal number of entries in
/// a Chord successor/predecessor table.
pub(crate) const CHORD_RING_TABLE_MAX_ENTRIES: usize = 16;

/// A transport-level address of a node in Chord.
pub(crate) type ChordAddr = usize;

/// A link identifier in Chord.
/// It comprises a node's identifier and
/// transport-level address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChordLinkId {
    pub(crate) id: ChordId,
    pub(crate) addr: ChordAddr,
}

/// A Chord node's routing state.
#[derive(Clone, Debug)]
pub(crate) struct ChordRoutingState {
    /// The finger table.
    pub(crate) finger_table: Vec<Option<ChordLinkId>>,
    /// The successor table.
    pub(crate) succ_table: Vec<Option<ChordLinkId>>,
    /// The predecessor table.
    pub(crate) pred_table: Vec<Option<ChordLinkId>>,
}

/// A message sent by Chord over the Internet.
/// (A wrapper over Chord message that in addition
/// carries transport-layer addresses.)
#[derive(Clone, Debug)]
pub(crate) struct ChordMessage {
    hdr: ChordMessageHeader,
    data: ChordMessageContent,
}

impl ChordMessage {
    pub(crate) fn new(
        dst_id: &ChordId,
        delivery_notifier: Sender<Vec<ChordId>>,
    ) -> Self {
        ChordMessage {
            hdr: ChordMessageHeader {
                dst_id: *dst_id,
            },
            data: ChordMessageContent {
                hops: Vec::new(),
                delivery_notifier,
            },
        }
    }
}

/// A header of a message sent by Chord over the Internet.
#[derive(Clone, Debug)]
pub(crate) struct ChordMessageHeader {
    dst_id: ChordId,
}

/// A content of a message sent by Chord over the Internet.
/// For demonstration purposes, it contains all hops
/// the message has followed and a channel for passing
/// this information back upon the delivery of the message.
#[derive(Clone, Debug)]
pub(crate) struct ChordMessageContent {
    hops: Vec<ChordId>,
    delivery_notifier: Sender<Vec<ChordId>>,
}

/// A module representing a node in Chord.
pub(crate) struct ChordNode {
    /// The node's identifier on the ring.
    id: ChordId,
    /// The node's transport-layer address.
    addr: ChordAddr,
    /// The node's routing state.
    rs: ChordRoutingState,
    /// The interface to the Internet (no need to use directly).
    net_ref: ModuleRef<Internet>,
    /// A reference to self (no need to use directly).
    self_ref: Arc<Mutex<Option<ModuleRef<ChordNode>>>>,
}

/// A Chord routing outcome.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ChordRoutingOutcome {
    /// Accepting a message by the routing node.
    Accept,
    /// Forwarding a message to the node with
    /// a given transport-layer address.
    Forward(ChordAddr),
}

impl ChordNode {
    pub(crate) async fn register(
        system: &mut System,
        net_ref: ModuleRef<Internet>,
        ring_bits: usize,
        // ring_redundancy is the parameter R from the learning section.
        ring_redundancy: usize,
        id: &ChordId,
        addr: &ChordAddr,
    ) -> ModuleRef<ChordNode> {
        assert!(ring_bits >= 1);
        assert!(ring_bits <= CHORD_FINGER_TABLE_MAX_ENTRIES);
        assert!(ring_redundancy >= 1);
        assert!(ring_redundancy <= CHORD_RING_TABLE_MAX_ENTRIES);
        assert!(id <= &chord_id_max(ring_bits));
        let node = Self {
            id: *id,
            addr: *addr,
            rs: ChordRoutingState {
                finger_table: vec![None; ring_bits],
                succ_table: vec![None; ring_redundancy],
                pred_table: vec![None; ring_redundancy],
            },
            net_ref: net_ref.clone(),
            self_ref: Arc::new(Mutex::new(None)),
        };
        let self_ref = node.self_ref.clone();
        let node_ref = system.register_module(node).await;
        let mut guard = self_ref.lock().await;
        *guard = Some(node_ref.clone());
        node_ref
    }

    /// For each Chord node, creates a complete routing
    /// state given (an oracle's) information about all
    /// nodes in the system, that is, a mapping
    /// `ChordId` -> `ChordAddr`.
    #[allow(clippy::len_zero)]
    pub(crate) fn recreate_links_from_oracle(&mut self, all_nodes: &BTreeMap<ChordId, ChordAddr>) {
        assert!(
            self.rs.finger_table.len() > 0
                && self.rs.finger_table.len() <= CHORD_FINGER_TABLE_MAX_ENTRIES
        );
        assert!(
            self.rs.succ_table.len() > 0
                && self.rs.succ_table.len() <= CHORD_RING_TABLE_MAX_ENTRIES
        );
        assert!(self.rs.pred_table.len() == self.rs.succ_table.len());
        assert!(all_nodes.contains_key(&self.id));
        assert!(
            all_nodes
                .iter()
                .filter(|(&k, &_v)| {
                    k < chord_id_min(self.rs.finger_table.len())
                        || k > chord_id_max(self.rs.finger_table.len())
                })
                .count()
                == 0
        );

        // FIXME: Implement this function. To this end, you may
        //        find the earlier chord_id_* functions useful.
        //        In essence, this function should only change
        //        self.rs, utilizing self.id and all_nodes.
        
        // system: &mut System,
        // net_ref: ModuleRef<Internet>,
        // ring_bits: usize, // B
        // ring_redundancy   // R
        //     /// The node's identifier on the ring.
        // id: ChordId,
        // /// The node's transport-layer address.
        // addr: ChordAddr,
        // /// The node's routing state.
        // rs: ChordRoutingState,
        // /// The interface to the Internet (no need to use directly).
        // net_ref: ModuleRef<Internet>,
        // /// A reference to self (no need to use directly).
        // self_ref: Arc<Mutex<Option<ModuleRef<ChordNode>>>>,
        let n = all_nodes.len();
        let r = self.rs.succ_table.len();
        let b = self.rs.finger_table.len();
        println!("n = {} r = {} b = {}", n, r, b);
        let mut chord_nodes: Vec<ChordId> = all_nodes.keys().cloned().collect();
        // double the ChordId vec in order to make computations easier
        chord_nodes = chord_nodes.iter().cloned().chain(chord_nodes.iter().cloned()).collect::<Vec<ChordId>>();
        // find idx of self.id in chard_nodes
        let mut idx: usize = 0;
        for i in 0..n {
            if chord_nodes[i] == self.id {
                idx = i;
                break;
            }
        }
        println!("idx = {}", idx);
        // fill succ_table
        let mut succ_idx = idx;
        for i in 0..r {
            succ_idx += 1;
            if succ_idx == n { succ_idx = 0; }
            if succ_idx == idx {
                self.rs.succ_table[i] = None;
            }
            else {
                self.rs.succ_table[i] = Some(ChordLinkId {
                    id: chord_nodes[succ_idx],
                    addr: *all_nodes.get(&chord_nodes[succ_idx]).unwrap()
                });
            }
        }
        // fill pred table
        let mut pred_idx = idx;
        for i in 0..r {
            if pred_idx == 0 { 
                pred_idx = n-1;
            } else {
                pred_idx -= 1;
            }
            if pred_idx == idx {
                self.rs.pred_table[i] = None;
            }
            else {
                self.rs.pred_table[i] = Some(ChordLinkId {
                    id: chord_nodes[pred_idx],
                    addr: *all_nodes.get(&chord_nodes[pred_idx]).unwrap()
                });
            }
        }
        // fill finger table
        let mut power: ChordId = 1;
        let mut next_idx = idx + 1;
        for i in 0..b {
            let lower_bound = chord_id_advance_by(b, &chord_nodes[idx], &power);
            let upper_bound = chord_id_advance_by(b, &lower_bound, &power);
            while next_idx < n+n &&
                  power > chord_id_distance(b, &chord_nodes[idx], &chord_nodes[next_idx])  
            {
                next_idx += 1;
            }
            if next_idx == n+n {
                self.rs.finger_table[i] = None;
            } else if chord_id_in_range(b, &chord_nodes[next_idx], lower_bound..upper_bound) {
                let normalised_idx = if next_idx >= n { 
                    next_idx-n
                } else { 
                    next_idx
                };
                self.rs.finger_table[i] = Some(ChordLinkId {
                    id: chord_nodes[normalised_idx],
                    addr: *all_nodes.get(&chord_nodes[normalised_idx]).unwrap()
                });
            } else {
                self.rs.finger_table[i] = None;
            }
            if i+1 != n {
                power *= 2;
            }
        }
    }

    /// Given a header of a Chord message, decides
    /// what routing step the processing node should
    /// perform, that is, whether to accept the
    /// message or forward it to another node.
    pub(crate) fn find_next_routing_hop(&self, hdr: &ChordMessageHeader) -> ChordRoutingOutcome {

        // FIXME: Implement this function. To this end, you may
        //        find the earlier chord_id_* functions useful.
        //        In essence, this function requires only
        //        self.rs, self.id, and hdr.dst_id.
        
        return ChordRoutingOutcome::Forward(12341234);
    }

    async fn recv_chord_msg(&mut self, msg: ChordMessage, _from_addr: &ChordAddr) {
        // Add self to the message as the next hop.
        let mut hops = msg.data.hops;
        hops.push(self.id);
        let new_msg = ChordMessage {
            hdr: msg.hdr,
            data: ChordMessageContent {
                hops,
                delivery_notifier: msg.data.delivery_notifier,
            },
        };
        // Route the message to self or another node.
        match self.find_next_routing_hop(&new_msg.hdr) {
            ChordRoutingOutcome::Accept => self.accept_chord_msg(new_msg).await,
            ChordRoutingOutcome::Forward(addr) => self.send_chord_msg(new_msg, &addr).await,
        };
    }

    async fn send_chord_msg(&self, msg: ChordMessage, to_addr: &ChordAddr) {
        let net_msg = InternetMessage {
            src: self.addr,
            dst: *to_addr,
            body: msg,
        };
        self.net_ref.send(net_msg).await;
    }

    async fn accept_chord_msg(&self, msg: ChordMessage) {
        msg.data
            .delivery_notifier
            .send(msg.data.hops)
            .await
            .unwrap();
    }

    #[cfg(test)]
    pub(crate) fn fetch_routing_state(&self) -> ChordRoutingState {
        self.rs.clone()
    }

    #[cfg(test)]
    pub(crate) fn replace_routing_state(&mut self, rs: ChordRoutingState) {
        self.rs = rs;
    }
}

/// The Internet.
/// It allows for sending `ChordMessages` between `ChordNodes`
/// given the nodes' `ChordAddrs`.
pub(crate) struct Internet {
    links: HashMap<ChordAddr, ModuleRef<ChordNode>>,
    self_ref: Arc<Mutex<Option<ModuleRef<Internet>>>>,
}

impl Internet {
    pub(crate) async fn register(system: &mut System) -> ModuleRef<Internet> {
        let net = Self {
            links: HashMap::new(),
            self_ref: Arc::new(Mutex::new(None)),
        };
        let self_ref = net.self_ref.clone();
        let net_ref = system.register_module(net).await;

        let mut guard = self_ref.lock().await;
        *guard = Some(net_ref.clone());

        net_ref
    }

    pub(crate) async fn connect_node(&mut self, addr: &ChordAddr, node_ref: &ModuleRef<ChordNode>) {
        match self.links.get(addr) {
            None => {
                self.links.insert(*addr, node_ref.clone());
            }
            Some(_) => {
                panic!("A node with address {} already exists!", addr);
            }
        }
    }
}

/// A transport-layer wrapper message
/// for a Chord message.
pub(crate) struct InternetMessage {
    src: ChordAddr,
    dst: ChordAddr,
    body: ChordMessage,
}

impl InternetMessage {
    pub(crate) fn new(src: &ChordAddr, dst: &ChordAddr, body: ChordMessage) -> Self {
        Self {
            src: *src,
            dst: *dst,
            body,
        }
    }
}

#[async_trait::async_trait]
impl Handler<InternetMessage> for Internet {
    async fn handle(&mut self, msg: InternetMessage) {
        if let Some(node) = self.links.get(&msg.dst) {
            node.send(msg).await;
        }
    }
}

#[async_trait::async_trait]
impl Handler<InternetMessage> for ChordNode {
    async fn handle(&mut self, msg: InternetMessage) {
        assert!(msg.dst == self.addr);
        self.recv_chord_msg(msg.body, &msg.src).await;
    }
}
