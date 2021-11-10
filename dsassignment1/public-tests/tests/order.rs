use assignment_1_solution::{Handler, System};
use tokio::sync::oneshot;
use ntest::timeout;

struct OrderlyModule {
    number: u128,
    final_number: u128,
    finished_sender: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NumberMsg {
    num: u128,
}

#[async_trait::async_trait]
impl Handler<NumberMsg> for OrderlyModule {
    async fn handle(&mut self, num_msg: NumberMsg) {
        if num_msg.num != self.number + 1 {
            panic!("Oh no bad order");
        }

        self.number = num_msg.num;

        if self.number == self.final_number {
            self.finished_sender.take().map(|s| s.send(()).unwrap());
        }
    }
}

#[tokio::test]
#[timeout(500)]
async fn message_order() {
    let final_number: u128 = 100_000;

    let mut sys = System::new().await;

    let (finished_sender, finished_receiver) = oneshot::channel();

    let module = OrderlyModule {number: 0, final_number, finished_sender: Some(finished_sender)};
    let mod_ref = sys.register_module(module).await;

    for i in 1..=final_number{
        mod_ref.send(NumberMsg{num: i}).await;
    }

    finished_receiver.await.unwrap();

    sys.shutdown().await;
}