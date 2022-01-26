use std::{collections::{HashSet, VecDeque}};

use executor::{Handler, ModuleRef, System};

/// Marker trait indicating that a broadcast implementation provides
/// guarantees specified in the assignment description.
pub(crate) trait ReliableBroadcast<const N: usize> {}

#[async_trait::async_trait]
pub(crate) trait ReliableBroadcastRef<const N: usize>: Send + Sync + 'static {
    async fn send(&self, msg: Operation);
}

#[async_trait::async_trait]
impl<T, const N: usize> ReliableBroadcastRef<N> for ModuleRef<T>
where
    T: ReliableBroadcast<N> + Handler<Operation> + Send,
{
    async fn send(&self, msg: Operation) {
        self.send(msg).await;
    }
}

/// Marker trait indicating that a client implementation
/// follows specification from the assignment description.
pub(crate) trait EditorClient {}

#[async_trait::async_trait]
pub(crate) trait ClientRef: Send + Sync + 'static {
    async fn send(&self, msg: Edit);
}

#[async_trait::async_trait]
impl<T> ClientRef for ModuleRef<T>
where
    T: EditorClient + Handler<Edit> + Send,
{
    async fn send(&self, msg: Edit) {
        self.send(msg).await;
    }
}

/// Actions (edits) which can be applied to a text.
#[derive(Clone)]
#[cfg_attr(test, derive(PartialEq, Debug))]
pub(crate) enum Action {
    /// Insert the character at the position.
    Insert { idx: usize, ch: char },
    /// Delete a character at the position.
    Delete { idx: usize },
    /// A _do nothing_ operation. `Nop` cannot be issued by a client.
    /// `Nop` can only be issued by a process or result from a transformation.
    Nop,

}

impl Action {
    /// Apply the action to the text.
    pub(crate) fn apply_to(&self, text: &mut String) {
        match self {
            Action::Insert { idx, ch } => {
                text.insert(*idx, *ch);
            }
            Action::Delete { idx } => {
                text.remove(*idx);
            }
            Action::Nop => {
                // Do nothing.
            }
        }
    }
}

/// Client's request to edit the text.
#[derive(Clone)]
pub(crate) struct EditRequest {
    /// Total number of operations a client has applied to its text so far.
    pub(crate) num_applied: usize,
    /// Action (edit) to be applied to a text.
    pub(crate) action: Action,
}

/// Response to a client with action (edit) it should apply to its text.
#[derive(Clone)]
pub(crate) struct Edit {
    pub(crate) action: Action,
}

#[derive(Clone)]
pub(crate) struct Operation {
    /// Rank of a process which issued this operation.
    pub(crate) process_rank: usize,
    /// Action (edit) to be applied to a text.
    pub(crate) action: Action,
}

impl Operation {
    // Add any methods you need.
}

pub(crate) async fn transform(op1: Operation, op2: Operation) -> Operation {
    let r1 = op1.process_rank;
    let r2 = op2.process_rank;
    match (op1.action.clone(), op2.action.clone()) {
        (Action::Nop, _) => {
            return op1; // NOP
        }
        (_, Action::Nop) => {
            return op1;
        }
        (Action::Insert { idx: p1, ch: _ }, Action::Insert { idx: p2, ch: c2 }) => {
            if p1 < p2 {
                return op1;    
            }
            if p1 == p2 && r1 < r2 {
                return op1;
            } else {
                return Operation {
                    action: Action::Insert {
                        idx: p1,
                        ch: c2,
                    },
                    process_rank: r1,
                }
            }
        }
        (Action::Delete { idx: p1 }, Action::Delete { idx: p2 }) => {
            if p1 < p2 {
                return op1;
            }
            if p1 == p2 {
                return Operation {
                    action: Action::Nop,
                    process_rank: r1
                };
            } else {
                return Operation {
                    action: Action::Delete {
                        idx: p1 - 1
                    },
                    process_rank: r1
                }
            }
        }
        (Action::Insert { idx: p1, ch: c1 }, Action::Delete { idx: p2 }) => {
            if p1 <= p2 {
                return op1;
            }
            else {
                return Operation {
                    action: Action::Insert {
                        idx: p1 - 1,
                        ch: c1
                    },
                    process_rank: r1
                }
            }
        }
        (Action::Delete { idx: p1 }, Action::Insert { idx: p2, ch: _ }) => {
            if p1 < p2 {
                return op1;
            }
            else {
                return Operation {
                    action: Action::Delete {
                        idx: p1 + 1
                    },
                    process_rank: r1
                }
            }
        }
    }
}

/// Process of the system.
pub(crate) struct Process<const N: usize> {
    /// Rank of the process.
    rank: usize,
    /// Reference to the broadcast module.
    broadcast: Box<dyn ReliableBroadcastRef<N>>,
    /// Reference to the process's client.
    client: Box<dyn ClientRef>,

    /// Process log
    log: Vec<Operation>,
    /// Processes that sent message in current round
    current_round_set: HashSet<usize>,
    /// Operations from processes from next round
    next_round_ops: Vec<Operation>,
    /// Queued messages from client 
    client_ops: VecDeque<EditRequest>,
}

impl<const N: usize> Process<N> {
    pub(crate) async fn new(
        system: &mut System,
        rank: usize,
        broadcast: Box<dyn ReliableBroadcastRef<N>>,
        client: Box<dyn ClientRef>,
    ) -> ModuleRef<Self> {
        let self_ref = system
            .register_module(Self {
                rank,
                broadcast,
                client,
                log: Vec::new(),
                current_round_set: HashSet::new(),
                next_round_ops: Vec::new(),
                client_ops: VecDeque::new(),
            })
            .await;
        self_ref
    }

    pub(crate) async fn apply_transform_from_idx(&mut self, op1: Operation, start_idx: usize) -> Operation {
        let mut op_ret = op1.clone();
        for op2 in self.log[start_idx..].iter().cloned() {
            op_ret = transform(op_ret, op2).await;
        }
        return op_ret;
    }

    pub(crate) async fn add_to_log_and_send_edit(&mut self, op: Operation, start_idx: usize) {
        let op_transformed = self.apply_transform_from_idx(op, start_idx).await;
        self.log.push(op_transformed.clone());
        match op_transformed.action {
            Action::Nop => {
                // do not send nop to client
            }
            _ => {
                self.client.send(Edit{ action: op_transformed.action}).await;
            }
        }
    }

    pub(crate) async fn first_client_edit_request(&mut self, request: EditRequest) {
        // client message is first message 
        let op = Operation { 
            process_rank: N + 1, 
            action: request.action.clone()
        };
        self.current_round_set.insert(self.rank);
        self.add_to_log_and_send_edit(op, request.num_applied).await;
        let op_broadcast = Operation {
            process_rank: self.rank,
            action: request.action.clone()
        };
        self.broadcast.send(op_broadcast).await;
    }

    pub(crate) async fn handle_op_from_process(&mut self, msg: Operation) {
        self.add_to_log_and_send_edit(msg.clone(), self.log.len() - self.current_round_set.len()).await; // todo +/- 1
        self.current_round_set.insert(msg.process_rank);
        if self.current_round_set.len() == N {
           self.start_new_round().await; 
        }
    }

    pub(crate) async fn start_new_round(&mut self) {
        self.current_round_set.clear();
        if self.client_ops.is_empty() {
            // NOP
            self.current_round_set.insert(self.rank);
            let nop_op = Operation {
                process_rank: self.rank,
                action: Action::Nop
            };
            self.add_to_log_and_send_edit(nop_op.clone(), self.log.len() - 1).await; // todo
            self.broadcast.send(nop_op).await;
        } else {
            // client message is first message
            let request = self.client_ops.pop_front();
            self.first_client_edit_request(request.unwrap()).await;
        }
        let next_round_ops_vec = self.next_round_ops.clone();
        self.next_round_ops = Vec::new();
        for op in next_round_ops_vec.iter().cloned() {
            let idx_start = self.log.len();
            self.add_to_log_and_send_edit(op, idx_start).await;
        }
    }

}


#[async_trait::async_trait]
impl<const N: usize> Handler<Operation> for Process<N> {
    async fn handle(&mut self, msg: Operation) {
        if self.current_round_set.contains(&msg.process_rank) {
            // message from next round
            self.next_round_ops.push(msg);
        } else {
            // message from current round
            self.handle_op_from_process(msg).await;
        }
    }
}

#[async_trait::async_trait]
impl<const N: usize> Handler<EditRequest> for Process<N> {
    async fn handle(&mut self, request: EditRequest) {
        if self.current_round_set.contains(&self.rank) {
            // already received message from client this round
            self.client_ops.push_back(request);
        } else {
            if self.current_round_set.is_empty() {
                // client message is first message
                self.first_client_edit_request(request).await;
            } else {
                println!("error, nie powinna taka sytuacja wystapic");
            } 
        } 
    }
}
