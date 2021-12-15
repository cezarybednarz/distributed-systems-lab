use async_channel::Sender;
use executor::{Handler, ModuleRef, System};
use std::convert::TryInto;
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

struct AssignModuleRefStore {
    module_ref: ModuleRef<CyberStore2047>
}

struct AssignModuleRefNode {
    module_ref: ModuleRef<Node>
}

/// Register and initialize a CyberStore2047 module.
pub(crate) async fn register_store(
    system: &mut System,
    store: CyberStore2047,
) -> ModuleRef<CyberStore2047> {
    let module_ref = system.register_module(store).await;
    module_ref.send(AssignModuleRefStore { module_ref: module_ref.clone() }).await;
    module_ref
}

/// Register and initialize a Node module.
pub(crate) async fn register_node(system: &mut System, node: Node) -> ModuleRef<Node> {
    let module_ref = system.register_module(node).await;
    module_ref.send(AssignModuleRefNode { module_ref: module_ref.clone() }).await;
    module_ref
}

/// CyberStore2047.
/// This structure serves as TM.
// Add any fields you need.
pub(crate) struct CyberStore2047 {
    module_ref: Option<ModuleRef<CyberStore2047>>,
    nodes: Vec<ModuleRef<Node>>,
    prepared_nodes: usize,
    commited_nodes: usize,
    aborted: bool,
    completed_callback: Option<Box<dyn FnOnce(TwoPhaseResult) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>,
}

impl CyberStore2047 {
    pub(crate) fn new(nodes: Vec<ModuleRef<Node>>) -> Self {
        Self {
            module_ref: None,
            nodes,
            prepared_nodes: 0,
            commited_nodes: 0,
            aborted: false,
            completed_callback: None,
        }
    }
}

/// Node of CyberStore2047.
/// This structure serves as a process of the distributed system.
// Add any fields you need.
pub(crate) struct Node {
    products: Vec<Product>,
    pending_transaction: Option<Transaction>,
    enabled: bool,
    module_ref: Option<ModuleRef<Node>>
}

impl Node {
    pub(crate) fn new(products: Vec<Product>) -> Self {
        Self {
            products: products.clone(),
            pending_transaction: None,
            enabled: true,
            module_ref: None,
        }
    }
}

#[async_trait::async_trait]
impl Handler<AssignModuleRefStore> for CyberStore2047 {
    async fn handle(&mut self, msg: AssignModuleRefStore) {
        self.module_ref = Some(msg.module_ref);
    }
}

#[async_trait::async_trait]
impl Handler<AssignModuleRefNode> for Node {
    async fn handle(&mut self, msg: AssignModuleRefNode) {
        self.module_ref = Some(msg.module_ref);
    }
}

#[async_trait::async_trait]
impl Handler<NodeMsg> for CyberStore2047 {
    async fn handle(&mut self, msg: NodeMsg) {
        match msg.content {
            NodeMsgContent::RequestVoteResponse(result) => {
                match result {
                    TwoPhaseResult::Ok => {
                        if !self.aborted {
                            self.prepared_nodes += 1;
                            if self.prepared_nodes == self.nodes.len() {
                                let mut tasks = Vec::new();
                                for node in &self.nodes {
                                    let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
                                    tasks.push(node.send(StoreMsg {
                                        sender: module_ref_clone,
                                        content: StoreMsgContent::Commit
                                    }));
                                }
                                for task in tasks {
                                    task.await;
                                }
                            }
                        }
                    }
                    TwoPhaseResult::Abort => {
                        if !self.aborted {
                            self.aborted = true;
                            let mut tasks = Vec::new();
                            for node in &self.nodes {
                                let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
                                tasks.push(node.send(StoreMsg {
                                    sender: module_ref_clone,
                                    content: StoreMsgContent::Abort
                                }));
                            }
                            for task in tasks {
                                task.await;
                            }
                        }
                    }
                }
            }
            NodeMsgContent::FinalizationAck => {
                self.commited_nodes += 1;
                if self.commited_nodes == self.nodes.len() {
                    let result = match self.aborted {
                        true => TwoPhaseResult::Abort,
                        false => TwoPhaseResult::Ok,
                    };
                    (self.completed_callback.take().unwrap())(result);
                    println!("callback! aborted:{}", self.aborted);
                    self.prepared_nodes = 0;
                    self.commited_nodes = 0;
                    self.aborted = false;
                    self.completed_callback = None;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Handler<StoreMsg> for Node {
    async fn handle(&mut self, msg: StoreMsg) {
        if self.enabled {
            match msg.content {
                StoreMsgContent::RequestVote(transaction) => {
                    self.pending_transaction = Some(transaction);
                    for product in self.products.iter_mut() {
                        if product.pr_type == transaction.pr_type {
                            let mut add_result = true;
                            if (transaction.shift <= 0 && (-transaction.shift) >= product.price.try_into().unwrap())
                                || (transaction.shift > 0 && product.price.checked_add(transaction.shift.try_into().unwrap()) == None) {
                                add_result = false;
                            }
                            let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
                            if add_result == true {
                                msg.sender.send(NodeMsg {
                                    sender: module_ref_clone,
                                    content: NodeMsgContent::RequestVoteResponse(TwoPhaseResult::Ok)
                                }).await;
                            } else {
                                msg.sender.send(NodeMsg {
                                    sender: module_ref_clone,
                                    content: NodeMsgContent::RequestVoteResponse(TwoPhaseResult::Abort)
                                }).await;
                            }
                        }
                    }
                }
                StoreMsgContent::Commit => {
                    for product in self.products.iter_mut() {
                        let transaction = self.pending_transaction.unwrap();
                        if product.pr_type == transaction.pr_type {
                            if transaction.shift <= 0 {
                                product.price += (-transaction.shift) as u64;
                            } else {
                                product.price += transaction.shift as u64;
                            }
                        }
                    }
                    self.pending_transaction = None;
                    let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
                    msg.sender.send(NodeMsg {
                        sender: module_ref_clone,
                        content: NodeMsgContent::FinalizationAck
                    }).await;
                }
                StoreMsgContent::Abort => {
                    self.pending_transaction = None;
                    let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
                    msg.sender.send(NodeMsg {
                        sender: module_ref_clone,
                        content: NodeMsgContent::FinalizationAck
                    }).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Handler<ProductPriceQuery> for Node {
    async fn handle(&mut self, msg: ProductPriceQuery) {
        if self.enabled {
            let mut found = false;
            for product in &self.products {
                if product.identifier == msg.product_ident {
                    found = true;
                    msg.result_sender.send(ProductPrice(Some(product.price))).await.unwrap();
                    break;
                }
            }
            if !found {
                msg.result_sender.send(ProductPrice(None)).await.unwrap();
            }
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
        self.completed_callback = Some(msg.completed_callback);
        let mut tasks = Vec::new();
        for node in &self.nodes {
            let module_ref_clone = self.module_ref.as_ref().unwrap().clone();
            tasks.push(node.send(StoreMsg {
                sender: module_ref_clone,
                content: StoreMsgContent::RequestVote(msg.transaction)
            }));
        }
        for task in tasks {
            task.await;
        }
    }
}
