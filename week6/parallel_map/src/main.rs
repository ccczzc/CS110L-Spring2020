use crossbeam_channel;
use std::{thread, time};

struct Mynum<T> {
    index: usize,
    value: T,
}

impl<T> Mynum<T> {
    pub fn new(index: usize, value: T) -> Self {
        Mynum { index: index, value: value }
    }
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    output_vec.resize_with(input_vec.len(), Default::default);
    // output_vec.
    // implement parallel map!
    let (sender1, receiver1) = crossbeam_channel::unbounded();
    let (sender2, receiver2) = crossbeam_channel::unbounded();
    let mut threads = Vec::new();
    for _ in 0..num_threads {
        let receiver: crossbeam_channel::Receiver<Mynum<T>> = receiver1.clone();
        let sender = sender2.clone();
        threads.push(thread::spawn(move || {
            while let Ok(cur_num) = receiver.recv() {
                sender.send(Mynum::new(cur_num.index, f(cur_num.value))).expect("Thread tried writing to channel, but there are no receivers!");
            }
        }))
    }

    for i in (0..input_vec.len()).rev() {
        sender1.send(Mynum::new(i, input_vec.pop().unwrap()))
                .expect("Tried writing to channel, but there are no receivers!");
    }
    drop(sender1);
    drop(sender2);
    while let Ok(cur_res) = receiver2.recv() {
        output_vec[cur_res.index] = cur_res.value;
    }

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
