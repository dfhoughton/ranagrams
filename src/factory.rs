//! thread model

use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, Builder};
use std::cmp;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Messages the monitor thread sends to worker threads to control their
/// activity.
#[derive(PartialEq, Eq, Debug)]
enum BossMessage {
    Go,
    Stop,
}

/// Messages worker threads send to the monitor thread to allow it to keep
/// track fo their state and distribute work.
#[derive(PartialEq, Eq, Debug)]
enum WorkerMessage {
    WakeUp,
    Slain,
    Sleeping(usize), // usize is an id indicating the sleeper
}

/// The resume (trait) required of worker threads who wish to work in the
/// factory. The general pattern of work is that workers inspect an item (`I`)
/// to see whether it can be shipped. If not, they improve it, making some
/// number of improved items.
pub trait WorkerFun<I: Send + 'static>: Send + Sync + 'static {
    fn improve(&self, I) -> Vec<I>;
    fn inspect(&self, &I) -> bool;
}

/// Start the factory going. The `roster` is the number of workers. The
/// `slop_factor` is multiplied by this number to determine the number of
/// items to keep in reserve for workers that run low in their personal work
/// queues. The `materials` are the initial items requiring improvement. The
/// `fun` provides the specifications for what the workers will do to improve
/// or inspect their work.
pub fn manufacture<I, W>(
    roster: usize,
    slop_factor: usize,
    materials: Vec<I>,
    fun: Arc<W>,
) -> (Receiver<Option<I>>, Arc<AtomicBool>)
where
    I: Send + 'static,
    W: WorkerFun<I>,
{
    // set up work sharing mechanism
    if roster == 0 {
        panic!("roster must be greater than 0");
    }
    if slop_factor == 0 {
        panic!("slop_factor must be greater than 0");
    }
    let maximum_shared = roster * slop_factor;
    let threshold = roster;
    let had = Arc::new(AtomicUsize::new(materials.len()));

    // set up factory floor
    let conveyor_belt = Arc::new(Mutex::new(materials));
    let (container, truck) = mpsc::channel::<Option<I>>();
    let workers = Arc::new(Mutex::new(Vec::with_capacity(roster)));
    let (manager, stamps) = mpsc::channel::<WorkerMessage>();
    let kill_switch = Arc::new(AtomicBool::new(false));
    for i in 0..roster {
        work(
            i,
            had.clone(),
            conveyor_belt.clone(),
            container.clone(),
            manager.clone(),
            fun.clone(),
            kill_switch.clone(),
            workers.clone(),
            threshold,
            maximum_shared,
        );
    }
    thread::spawn(move || supervize(roster, workers, stamps, container));
    (truck, kill_switch)
}

fn work<I, W>(
    i: usize,
    had: Arc<AtomicUsize>,
    belt: Arc<Mutex<Vec<I>>>,
    container: Sender<Option<I>>,
    manager: Sender<WorkerMessage>,
    fun: Arc<W>,
    kill_switch: Arc<AtomicBool>,
    workers: Arc<Mutex<Vec<Sender<BossMessage>>>>,
    threshold: usize,
    maximum_shared: usize,
) where
    I: Send + 'static,
    W: WorkerFun<I>,
{
    let (worker, in_box) = mpsc::channel::<BossMessage>();
    workers.lock().unwrap().push(worker);
    let bob = Builder::new().name(format!("{}", i).into());
    bob.spawn(move || {
        let mut hopper = vec![];
        for message in in_box {
            if message == BossMessage::Stop {
                break;
            }
            if kill_switch.load(Ordering::Relaxed) {
                manager.send(WorkerMessage::Slain).ok();
                break;
            }
            while let Some(stuff) = {
                let mut temp = belt.lock().unwrap();
                temp.pop()
            } {
                // push the stuff into the owned queue and work off that
                hopper.push(stuff);
                had.fetch_sub(1, Ordering::Relaxed);
                while let Some(stuff) = hopper.pop() {
                    if kill_switch.load(Ordering::Relaxed) {
                        manager.send(WorkerMessage::Slain).ok();
                        break;
                    }
                    if fun.inspect(&stuff) {
                        container.send(Some(stuff)).ok();
                    } else {
                        let mut widgets = fun.improve(stuff);
                        let currently_shared = had.load(Ordering::Relaxed);
                        if currently_shared < threshold {
                            let own = widgets.len() + hopper.len();
                            if own > 1 {
                                let mut tithe =
                                    cmp::min(own - 1, maximum_shared - currently_shared);
                                had.fetch_add(tithe, Ordering::Relaxed);
                                let mut belt = belt.lock().unwrap();
                                if widgets.len() > 0 {
                                    if widgets.len() <= tithe {
                                        tithe -= widgets.len();
                                        for _ in 0..widgets.len() {
                                            belt.push(widgets.pop().unwrap());
                                        }
                                    } else {
                                        for _ in 0..tithe {
                                            belt.push(widgets.pop().unwrap());
                                        }
                                        tithe = 0;
                                    }
                                }
                                for _ in 0..tithe {
                                    belt.push(hopper.pop().unwrap());
                                }
                                manager.send(WorkerMessage::WakeUp).ok();
                            }
                        }
                        for w in widgets {
                            hopper.push(w);
                        }
                    }
                }
            }
            manager.send(WorkerMessage::Sleeping(i)).ok(); // send I'm empty message
        }
    }).unwrap();
}

fn supervize<I>(
    roster: usize,
    workers: Arc<Mutex<Vec<Sender<BossMessage>>>>,
    stamps: Receiver<WorkerMessage>,
    container: Sender<Option<I>>,
) where
    I: Send + 'static,
{
    let mut idled: Vec<usize> = Vec::with_capacity(roster);
    for w in workers.lock().unwrap().iter() {
        w.send(BossMessage::Go).ok();
    }
    for message in stamps {
        match message {
            WorkerMessage::Slain => {
                container.send(None).ok();
                let foo = workers.lock().unwrap();
                for &i in idled.iter() {
                    if let Some(w) = foo.get(i) {
                        w.send(BossMessage::Go).ok();
                    }
                }
                break;
            }
            WorkerMessage::WakeUp => {
                let foo = workers.lock().unwrap();
                for &i in idled.iter() {
                    if let Some(w) = foo.get(i) {
                        w.send(BossMessage::Go).ok();
                    }
                }
                idled.clear();
            }
            WorkerMessage::Sleeping(i) => {
                idled.push(i);
                if idled.len() == roster {
                    container.send(None).ok();
                    for worker in workers.lock().unwrap().iter() {
                        worker.send(BossMessage::Stop).ok();
                    }
                }
            }
        }
    }
}
