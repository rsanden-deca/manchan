use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

struct Inner<T> {
    queue: VecDeque<T>,
    n_senders: usize,
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

pub fn new_channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: VecDeque::<T>::new(),
        n_senders: 1,
    };
    let shared = Shared {
        inner: Mutex::new(inner),
        available: Condvar::new(),
    };
    let arc_shared = Arc::new(shared);
    let tx = Sender {
        shared: arc_shared.clone(),
    };
    let rx = Receiver {
        shared: arc_shared.clone(),
    };
    (tx, rx)
}

impl<T> Sender<T> {
    pub fn send(&mut self, msg: T) {
        let mut inner_guard = self.shared.inner.lock().unwrap();
        inner_guard.queue.push_back(msg);
        self.shared.available.notify_one();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner_guard = self.shared.inner.lock().unwrap();
        inner_guard.n_senders += 1;
        drop(inner_guard);
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner_guard = self.shared.inner.lock().unwrap();
        inner_guard.n_senders -= 1;
        let is_channel_close = inner_guard.n_senders == 0;
        drop(inner_guard);
        if is_channel_close {
            self.shared.available.notify_all();
        }
    }
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        let mut inner_guard = self.shared.inner.lock().unwrap();
        loop {
            if let Some(val) = inner_guard.queue.pop_front() {
                return Some(val);
            }
            if inner_guard.n_senders == 0 {
                return None; // channel is closed
            }
            inner_guard = self.shared.available.wait(inner_guard).unwrap();
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Iterator for Receiver<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{self, sleep};
    use std::time::Duration;

    #[test]
    fn test_channel_pingpong() {
        let (mut tx, mut rx) = new_channel();
        tx.send("hello".to_string());
        tx.send("world".to_string());
        assert_eq!(rx.recv(), Some("hello".to_string()));
        assert_eq!(rx.recv(), Some("world".to_string()));
    }

    #[test]
    fn test_channel_iterator() {
        let (mut tx, rx) = new_channel();
        for i in 0..5 {
            tx.send(i);
        }
        drop(tx);

        for (i, val) in rx.enumerate() {
            match i {
                0 => assert_eq!(val, 0),
                1 => assert_eq!(val, 1),
                2 => assert_eq!(val, 2),
                3 => assert_eq!(val, 3),
                4 => assert_eq!(val, 4),
                _ => panic!("bad length"),
            }
        }
    }

    #[test]
    fn test_channel_concurrent() {
        let (mut tx, mut rx) = new_channel();

        let f = move || {
            for i in 0..5 {
                tx.send(format!("hello {}", i));
                sleep(Duration::new(0, 10000000));
            }
        };
        thread::spawn(f);

        for i in 0..5 {
            assert_eq!(rx.recv(), Some(format!("hello {}", i)));
        }
    }

    #[test]
    fn test_channel_mpsc() {
        let (tx, mut rx) = new_channel();
        let mut tx1 = tx.clone();
        let mut tx2 = tx.clone();
        let mut tx3 = tx.clone();
        drop(tx);

        thread::spawn(move || {
            for i in 0..5 {
                tx1.send(format!("hello {} from {}", i, 1));
                sleep(Duration::new(0, 10000000));
            }
        });
        thread::spawn(move || {
            for i in 0..5 {
                tx2.send(format!("hello {} from {}", i, 2));
                sleep(Duration::new(0, 20000000));
            }
        });
        thread::spawn(move || {
            for i in 0..5 {
                tx3.send(format!("hello {} from {}", i, 3));
                sleep(Duration::new(0, 30000000));
            }
        });

        for _ in 0..15 {
            assert!(rx.recv().is_some());
        }
        assert!(rx.recv().is_none());
    }

    #[test]
    fn test_channel_spmc() {
        let (mut tx, rx) = new_channel();
        let mut rx1 = rx.clone();
        let mut rx2 = rx.clone();
        let mut rx3 = rx.clone();
        drop(rx);

        let rx1_handle = thread::spawn(move || {
            let mut rx1_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx1_results.push(rx1.recv());
                sleep(Duration::new(0, 10000000));
            }
            rx1_results
        });
        let rx2_handle = thread::spawn(move || {
            let mut rx2_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx2_results.push(rx2.recv());
                sleep(Duration::new(0, 10000000));
            }
            rx2_results
        });
        let rx3_handle = thread::spawn(move || {
            let mut rx3_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx3_results.push(rx3.recv());
                sleep(Duration::new(0, 10000000));
            }
            rx3_results
        });

        for i in 0..15 {
            tx.send(format!("hello #{:02}", i));
        }
        let mut rx1_results = rx1_handle.join().unwrap();
        let mut rx2_results = rx2_handle.join().unwrap();
        let mut rx3_results = rx3_handle.join().unwrap();

        let mut results: Vec<Option<String>> = vec![];
        results.append(&mut rx1_results);
        results.append(&mut rx2_results);
        results.append(&mut rx3_results);
        let mut sorted_results = results
            .into_iter()
            .map(|item| item.unwrap())
            .collect::<Vec<_>>();
        sorted_results.sort();

        assert_eq!(
            sorted_results,
            vec![
                "hello #00".to_string(),
                "hello #01".to_string(),
                "hello #02".to_string(),
                "hello #03".to_string(),
                "hello #04".to_string(),
                "hello #05".to_string(),
                "hello #06".to_string(),
                "hello #07".to_string(),
                "hello #08".to_string(),
                "hello #09".to_string(),
                "hello #10".to_string(),
                "hello #11".to_string(),
                "hello #12".to_string(),
                "hello #13".to_string(),
                "hello #14".to_string(),
            ]
        )
    }

    #[test]
    fn test_channel_mpmc() {
        let (tx, mut rx) = new_channel();
        let mut tx1 = tx.clone();
        let mut tx2 = tx.clone();
        let mut tx3 = tx.clone();
        drop(tx);

        let mut rx1 = rx.clone();
        let mut rx2 = rx.clone();
        let mut rx3 = rx.clone();

        let rx1_handle = thread::spawn(move || {
            let mut rx1_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx1_results.push(rx1.recv());
                sleep(Duration::new(0, 10000000));
            }
            (rx1, rx1_results)
        });
        let rx2_handle = thread::spawn(move || {
            let mut rx2_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx2_results.push(rx2.recv());
                sleep(Duration::new(0, 10000000));
            }
            (rx2, rx2_results)
        });
        let rx3_handle = thread::spawn(move || {
            let mut rx3_results: Vec<Option<String>> = vec![];
            for _ in 0..5 {
                rx3_results.push(rx3.recv());
                sleep(Duration::new(0, 10000000));
            }
            (rx3, rx3_results)
        });

        let tx1_handle = thread::spawn(move || {
            for i in 0..5 {
                tx1.send(format!("hello #{} from tx1", i));
                sleep(Duration::new(0, 11000000));
            }
        });
        let tx2_handle = thread::spawn(move || {
            for i in 0..5 {
                tx2.send(format!("hello #{} from tx2", i));
                sleep(Duration::new(0, 13000000));
            }
        });
        let tx3_handle = thread::spawn(move || {
            for i in 0..5 {
                tx3.send(format!("hello #{} from tx3", i));
                sleep(Duration::new(0, 15000000));
            }
        });
        let _ = tx1_handle.join().unwrap();
        let _ = tx2_handle.join().unwrap();
        let _ = tx3_handle.join().unwrap();
        let (mut rx1, mut rx1_results) = rx1_handle.join().unwrap();
        let (mut rx2, mut rx2_results) = rx2_handle.join().unwrap();
        let (mut rx3, mut rx3_results) = rx3_handle.join().unwrap();

        let mut results: Vec<Option<String>> = vec![];
        results.append(&mut rx1_results);
        results.append(&mut rx2_results);
        results.append(&mut rx3_results);
        let mut sorted_results = results
            .into_iter()
            .map(|item| item.unwrap())
            .collect::<Vec<_>>();
        sorted_results.sort();

        assert_eq!(
            sorted_results,
            vec![
                "hello #0 from tx1".to_string(),
                "hello #0 from tx2".to_string(),
                "hello #0 from tx3".to_string(),
                "hello #1 from tx1".to_string(),
                "hello #1 from tx2".to_string(),
                "hello #1 from tx3".to_string(),
                "hello #2 from tx1".to_string(),
                "hello #2 from tx2".to_string(),
                "hello #2 from tx3".to_string(),
                "hello #3 from tx1".to_string(),
                "hello #3 from tx2".to_string(),
                "hello #3 from tx3".to_string(),
                "hello #4 from tx1".to_string(),
                "hello #4 from tx2".to_string(),
                "hello #4 from tx3".to_string(),
            ]
        );
        assert_eq!(rx.recv(), None);
        assert_eq!(rx1.recv(), None);
        assert_eq!(rx2.recv(), None);
        assert_eq!(rx3.recv(), None);
    }
}
