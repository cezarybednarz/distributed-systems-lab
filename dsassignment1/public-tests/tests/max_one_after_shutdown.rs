use assignment_1_solution::{Handler, System, ModuleRef};
use ntest::timeout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

struct MyMod {
    shutting_down: Arc<AtomicBool>,
    handled_when_shutting_down: u128,
    ok_sender: Option<oneshot::Sender<bool>>,
}

struct MyMsg;

#[async_trait::async_trait]
impl Handler<MyMsg> for MyMod {
    async fn handle(&mut self, _msg: MyMsg) {
        if self.shutting_down.load(Ordering::Relaxed) {
            self.handled_when_shutting_down += 1;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

impl Drop for MyMod {
    fn drop(&mut self) {
        let is_ok: bool = self.handled_when_shutting_down < 1;
        self.ok_sender.take().map(|s| s.send(is_ok));
    }
}


#[tokio::test]
#[timeout(1000)]
async fn max_one_after_shutdown() {
    let mut sys = System::new().await;

    let shutting_down: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let mut module_refs: Vec<ModuleRef<MyMod>> = Vec::new();
    let mut ok_receivers: Vec<oneshot::Receiver<bool>> = Vec::new();
    for _ in 0..64 {
        let (ok_sender, ok_receiver) = oneshot::channel::<bool>();
        let module = MyMod {shutting_down: shutting_down.clone(), handled_when_shutting_down: 0, ok_sender: Some(ok_sender)};
        module_refs.push(sys.register_module(module).await);
        ok_receivers.push(ok_receiver);
    }

    for mod_ref in &module_refs {
        for _ in 0..64 {
            mod_ref.send(MyMsg).await;
        }
    }

    shutting_down.store(true, Ordering::Relaxed);
    sys.shutdown().await;

    for ok_receiver in ok_receivers {
        let is_ok: bool = ok_receiver.await.unwrap();
        assert!(is_ok);
    }
}