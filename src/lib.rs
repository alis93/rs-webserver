use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// This is used to allow a function to take ownership of a boxed value.
// According to docs, this won't be needed in future (HOPEFULLY!)
trait FnBox {
    fn call_box(self: Box<Self>);
}


impl<F> FnBox for F
where
    F: FnOnce(),
{
    fn call_box(self: Box<F>) {
        (*self)();
    }
}

type Job = Box<dyn FnBox + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    queue: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero or less.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let mut workers = Vec::with_capacity(size);
        let (tx, rx) = mpsc::channel();

        let rx = Arc::new(Mutex::new(rx));

        for id in 0..size {
            workers.push(Worker::new(id, rx.clone()));
        }
        ThreadPool { workers, queue: tx }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::NewJob(Box::new(f));
        &self.queue.send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Send termination message to workers.
        // two loops required since they wont receive message in order.
        for _ in &mut self.workers {
            self.queue.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                println!("Shutting down worker {}", worker.id);

                thread.join().unwrap();
            }
        }
    }
}

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // get reference to mutex. lock it. receive job.
                let message: Message = receiver.as_ref().lock().unwrap().recv().unwrap();
                match message {
                    Message::NewJob(j) => {
                        println!("Worker {} got a job; executing.", id);
                        j.call_box()
                    }
                    Message::Terminate => {
                        println!("Worker {} terminating.", id);
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
