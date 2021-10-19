mod public_test;
mod solution;

use std::sync::{Arc, Mutex};

fn test() {
    let array_on_heap: Box<[u32]> = Box::new([1, 2, 3, 4, 5, 42]);
    println!("An array stored on the heap: {:?}", array_on_heap);
}

fn main() {
    test();

    // let shared_vec = Arc::new(Mutex::new(Vec::new()));
    // let pool = solution::Threadpool::new(2);

    // for x in 0..6 {
    //     let shared_vec_clone = shared_vec.clone();
    //     pool.submit(Box::new(move || {
    //         std::thread::sleep(std::time::Duration::from_millis(500));
    //         let mut vec = shared_vec_clone.lock().unwrap();
    //         vec.push(x);
    //         println!("Data: {:#?}", vec);
    //     }));
    // }
}
