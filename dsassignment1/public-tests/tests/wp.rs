mod fibonacci {
  use assignment_1_solution::{Handler, ModuleRef, Tick};
  use async_channel::Sender;
  use std::time::Duration;
  use tokio::time::sleep;

  pub type Num = u64;
  pub type Ident = ModuleRef<FibonacciModule>;

  pub struct FibonacciModule {
      name: &'static str,
      /// Currently hold number from the sequence.
      num: Num,
      /// Index of the required Fibonacci number (the `n`).
      limit: usize,
      /// Identifier of the other module.
      other: Option<Ident>,
      /// Result pipe.
      pipe: Sender<Num>,
  }

  impl FibonacciModule {
      /// Create the module and register it in the system.
      pub fn new(
          name: &'static str,
          initial_number: Num,
          limit: usize,
          pipe: Sender<Num>,
      ) -> FibonacciModule {
          FibonacciModule {
              name,
              num: initial_number,
              limit,
              other: None,
              pipe,
          }
      }

      /// Handle the step-message from the other module.
      ///
      /// Here the next number of the Fibonacci sequence is calculated.
      async fn message(&mut self, idx: usize, num: Num) {
          if idx >= self.limit {
              // The calculation is done.
              self.other
                  .as_ref()
                  .unwrap()
                  .send(FibonacciSystemMessage::Done)
                  .await;
              self.pipe.send(num).await.unwrap();
              return;
          }

          self.num = self.num + num;
          println!("Inside, value: {}", self.num);

          if idx < self.limit {
              self.other
                  .as_ref()
                  .unwrap()
                  .send(FibonacciSystemMessage::Message {
                      idx: idx + 1,
                      num: self.num,
                  })
                  .await;
          }
      }

      /// Handle the init-message.
      ///
      /// The module finishes its initialization and initiates the calculation
      /// if it is the first to go.
      async fn init(&mut self, other: Ident) {
          self.other = Some(other);
          if self.num == 1 {
              self.other
                  .as_ref()
                  .unwrap()
                  .send(FibonacciSystemMessage::Message {
                      idx: 1,
                      num: self.num,
                  })
                  .await;
          }
      }
  }

  /// Messages sent to/from the modules.
  ///
  /// The `id` field denotes which module should receive the message.
  pub enum FibonacciSystemMessage {
      /// Finalize module initialization and initiate the calculations.
      Init { other: Ident },

      /// Initiate the next step of the calculations.
      ///
      /// `idx` is the index in the sequence.
      /// `num` is the current number of the sequence.
      Message { idx: usize, num: Num },

      /// Indicate the end of calculations.
      Done,
  }

  #[async_trait::async_trait]
  impl Handler<FibonacciSystemMessage> for FibonacciModule {
      async fn handle(&mut self, msg: FibonacciSystemMessage) {
          sleep(Duration::from_micros(100)).await;
          match msg {
              FibonacciSystemMessage::Init { other } => {
                  self.init(other).await;
              }
              FibonacciSystemMessage::Message { idx, num } => {
                  self.message(idx, num).await;
              }
              FibonacciSystemMessage::Done => {}
          }
      }
  }

  #[async_trait::async_trait]
  impl Handler<Tick> for FibonacciModule {
      async fn handle(&mut self, _: Tick) {
          //println!("{} received tick.", self.name);
      }
  }

  pub fn fib_helper(k: usize) -> Num {
      let mut a: Num = 0;
      let mut b: Num = 1;
      for _ in 1..k {
          let result = a + b;
          a = b;
          b = result;
      }

      b
  }
}

use assignment_1_solution::System;
use async_channel::unbounded;
use fibonacci::{fib_helper, FibonacciModule, FibonacciSystemMessage};
use ntest::timeout;
use tokio::time::Duration;

#[allow(non_upper_case_globals)]
const n: usize = 20;

/// Calculate the `n`-th Fibonacci number.
#[tokio::test]
#[timeout(300)]
async fn fib() {
  let mut sys = System::new().await;

  let (pipe, result) = unbounded();

  let fib1 = sys
      .register_module(FibonacciModule::new("fib1", 0, n, pipe.clone()))
      .await;
  let fib2 = sys
      .register_module(FibonacciModule::new("fib2", 1, n, pipe))
      .await;

  sys.request_tick(&fib1, Duration::from_nanos(100)).await;
  sys.request_tick(&fib2, Duration::from_nanos(70)).await;
  sys.request_tick(&fib2, Duration::from_nanos(130)).await;

  fib1.send(FibonacciSystemMessage::Init {
      other: fib2.clone(),
  })
  .await;

  fib2.send(FibonacciSystemMessage::Init { other: fib1 })
      .await;

  let control_sample = fib_helper(n);

  let res = result.recv().await.unwrap();
  println!(">>> {}", control_sample);
  println!(">>> {}", res);
  assert_eq!(res, control_sample);

  sys.shutdown().await;
}