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

/// Process of the system.
pub(crate) struct Process<const N: usize> {
    /// Rank of the process.
    rank: usize,
    /// Reference to the broadcast module.
    broadcast: Box<dyn ReliableBroadcastRef<N>>,
    /// Reference to the process's client.
    client: Box<dyn ClientRef>,

    // Add any fields you need.
}

impl<const N: usize> Process<N> {
    pub(crate) async fn new(
        system: &mut System,
        rank: usize,
        broadcast: Box<dyn ReliableBroadcastRef<N>>,
        client: Box<dyn ClientRef>,
        // log
        // Set // wiemy ktore wrzucic do kolejnej rundy i kiedy zaczac nowa runde
        // kolejka od klientów
        // kolejka od innych broadcastów z rundy 
        // kolejka od innych broadcastów z kolejnej rundy
    ) -> ModuleRef<Self> {
        let self_ref = system
            .register_module(Self {
                rank,
                broadcast,
                client,
                // Add any fields you need.
            })
            .await;
        self_ref
    }

    // Add any methods you need.
    // todo start_new_round()

}

async fn transform(op1: Operation, op2: Operation) -> Operation {
    let r1 = op1.process_rank;
    let r2 = op2.process_rank;
    match (op1.action.clone(), op2.action.clone()) {

        (Action::Nop, _) => {
            return op2;
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

#[async_trait::async_trait]
impl<const N: usize> Handler<Operation> for Process<N> {
    async fn handle(&mut self, msg: Operation) {
        todo!("Handle operation issued by other process.");
        // dodaj na kolejke broadcastowa

        // przelicz op1 transformem
        
        // jesli nowa runda to start_new_round (wykonaj te 3 ify od kacpra // z wiadomosci 
    }
}

#[async_trait::async_trait]
impl<const N: usize> Handler<EditRequest> for Process<N> {
    async fn handle(&mut self, request: EditRequest) {
        todo!("Handle edit request from the client.");
        // dodaj na kolejke clientową
        
        // przelicz op1 transformrem i odeslij do kleinta Edit
        // przeliczamy transform zlozenie log[request.num_applied..]
    }
}
