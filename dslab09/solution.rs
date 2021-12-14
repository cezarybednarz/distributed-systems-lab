use async_channel::Sender;
use executor::{Handler, ModuleRef, System};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum ProductType {
    Electronics,
    Toys,
    Books,
}

#[derive(Clone)]
pub(crate) struct StoreMsg {
    sender: ModuleRef<CyberStore2047>,
    content: StoreMsgContent,
}

#[derive(Clone, Debug)]
pub(crate) enum StoreMsgContent {
    /// Transaction Manager initiates voting for the transaction.
    RequestVote(Transaction),
    /// If every process is ok with transaction, TM issues commit.
    Commit,
    /// System-wide abort.
    Abort,
}

#[derive(Clone)]
pub(crate) struct NodeMsg {
    sender: ModuleRef<Node>,
    content: NodeMsgContent,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeMsgContent {
    /// Process replies to TM whether it can/cannot commit the transaction.
    RequestVoteResponse(TwoPhaseResult),
    /// Process acknowledges to TM committing/aborting the transaction.
    FinalizationAck,
}

pub(crate) struct TransactionMessage {
    /// Request to change price.
    pub(crate) transaction: Transaction,

    /// Called after 2PC completes (i.e., the transaction was decided to be
    /// committed/aborted by CyberStore2047). This must be called after responses
    /// from all processes acknowledging commit or abort are collected.
    pub(crate) completed_callback:
        Box<dyn FnOnce(TwoPhaseResult) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TwoPhaseResult {
    Ok,
    Abort,
}

#[derive(Copy, Clone)]
pub(crate) struct Product {
    pub(crate) identifier: Uuid,
    pub(crate) pr_type: ProductType,
    pub(crate) price: u64,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Transaction {
    pub(crate) pr_type: ProductType,
    pub(crate) shift: i32,
}

pub(crate) struct ProductPriceQuery {
    pub(crate) product_ident: Uuid,
    pub(crate) result_sender: Sender<ProductPrice>,
}

pub(crate) struct ProductPrice(pub(crate) Option<u64>);

/// Message which disables a node. Used for testing.
pub(crate) struct Disable;

/// Register and initialize a CyberStore2047 module.
pub(crate) async fn register_store(
    system: &mut System,
    store: CyberStore2047,
) -> ModuleRef<CyberStore2047> {
    unimplemented!()
}

/// Register and initialize a Node module.
pub(crate) async fn register_node(system: &mut System, node: Node) -> ModuleRef<Node> {
    unimplemented!()
}

/// CyberStore2047.
/// This structure serves as TM.
// Add any fields you need.
pub(crate) struct CyberStore2047 {}

impl CyberStore2047 {
    pub(crate) fn new(nodes: Vec<ModuleRef<Node>>) -> Self {
        unimplemented!()
    }
}

/// Node of CyberStore2047.
/// This structure serves as a process of the distributed system.
// Add any fields you need.
pub(crate) struct Node {
    products: Vec<Product>,
    pending_transaction: Option<Transaction>,
    enabled: bool,
}

impl Node {
    pub(crate) fn new(products: Vec<Product>) -> Self {
        Self {
            products,
            pending_transaction: None,
            enabled: true,
        }
    }
}

#[async_trait::async_trait]
impl Handler<NodeMsg> for CyberStore2047 {
    async fn handle(&mut self, msg: NodeMsg) {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl Handler<StoreMsg> for Node {
    async fn handle(&mut self, msg: StoreMsg) {
        if self.enabled {
            unimplemented!()
        }
    }
}

#[async_trait::async_trait]
impl Handler<ProductPriceQuery> for Node {
    async fn handle(&mut self, msg: ProductPriceQuery) {
        if self.enabled {
            unimplemented!()
        }
    }
}

#[async_trait::async_trait]
impl Handler<Disable> for Node {
    async fn handle(&mut self, _msg: Disable) {
        self.enabled = false;
    }
}

#[async_trait::async_trait]
impl Handler<TransactionMessage> for CyberStore2047 {
    async fn handle(&mut self, msg: TransactionMessage) {
        unimplemented!()
    }
}
