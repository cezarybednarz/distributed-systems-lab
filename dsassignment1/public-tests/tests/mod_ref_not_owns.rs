use assignment_1_solution::System;
use tokio::task::JoinHandle;
use ntest::timeout;

struct MyMod {
    ping_task: JoinHandle<()>,
}

impl MyMod {
    fn new(ping_receiver: async_channel::Receiver<()>, pong_sender: async_channel::Sender<()>) -> MyMod {
        let ping_task = tokio::spawn(async move {
            while let Ok(()) = ping_receiver.recv().await {
                let _ = pong_sender.send(()).await;
            }
        });

        MyMod {ping_task}
    }
}

impl Drop for MyMod {
    fn drop(&mut self) {
        println!("DROPPOING");
        self.ping_task.abort();
    }
}

// I don't know if this required, but seems nice to have
#[ignore] // <- Remove this to enable
#[tokio::test]
#[timeout(500)]
async fn mod_ref_not_owns() {
    let mut sys = System::new().await;

    let (ping_sender, ping_receiver) = async_channel::unbounded();
    let (pong_sender, pong_receiver) = async_channel::unbounded();

    let module = MyMod::new(ping_receiver, pong_sender);
    let mod_ref = sys.register_module(module).await;
    std::mem::drop(mod_ref);
    println!("WTF");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("PINGING");

    ping_sender.send(()).await.unwrap();
    let _pong: () = pong_receiver.recv().await.unwrap();

    sys.shutdown().await;
}