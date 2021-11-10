use assignment_1_solution::{Handler, System};
use ntest::timeout;

struct MyMod {
    sender: async_channel::Sender<()>
}

struct MyMsg;

#[async_trait::async_trait]
impl Handler<MyMsg> for MyMod {
    async fn handle(&mut self, _msg: MyMsg) {
        self.sender.send(()).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.sender.send(()).await.unwrap();
    }
}

#[tokio::test]
#[timeout(500)]
async fn shutdown_completes_handle() {
    let mut sys = System::new().await;

    let (sender, receiver) = async_channel::unbounded::<()>();

    let module = MyMod {sender};
    let mod_ref = sys.register_module(module).await;

    for _ in 0..100 {
        mod_ref.send(MyMsg).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(125)).await;
    sys.shutdown().await;

    let mut received_num: u128 = 0;
    while let Ok(()) = receiver.recv().await {
        received_num += 1;
    }

    assert!(received_num > 0);
    assert_eq!(received_num % 2, 0);
}
