use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::cmp;
use std::sync::mpsc::Receiver;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(PartialEq, Eq, Debug)]
enum BossMessage {
    Go,
    Stop,
}

#[derive(PartialEq, Eq, Debug)]
enum WorkerMessage {
    WakeUp,
    Slain,
    Sleeping(usize),
}

pub trait WorkerFun<I: Send + 'static>: Send + Sync + 'static {
    fn improve(&self, I) -> Vec<I>;
    fn inspect(&self, &I) -> bool;
}

pub fn manufacture<I, W>(
    roster: usize,
    materials: Vec<I>,
    fun: Arc<W>,
) -> (Receiver<Option<I>>, Arc<AtomicBool>)
where
    I: Send + 'static,
    W: WorkerFun<I>,
{
    let mut wanting: isize = (roster as isize) - (materials.len() as isize);
    if wanting < 0 {
        wanting = 0;
    }
    let wanted = Arc::new(AtomicUsize::new(wanting as usize));
    let conveyor_belt = Arc::new(Mutex::new(materials));
    let (container, truck) = mpsc::channel::<Option<I>>();
    let workers = Arc::new(Mutex::new(Vec::with_capacity(roster)));
    let (manager, stamps) = mpsc::channel::<WorkerMessage>();
    let kill_switch = Arc::new(AtomicBool::new(false));
    (0..roster)
        .map(|i| {
            let wanted = wanted.clone();
            let belt = conveyor_belt.clone();
            let container = container.clone();
            let manager = manager.clone();
            let fun = fun.clone();
            let (worker, in_box) = mpsc::channel::<BossMessage>();
            let kill_switch = kill_switch.clone();
            workers.lock().unwrap().push(worker);
            thread::spawn(move || {
                let mut inbox = vec![];
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
                        inbox.push(stuff);
                        while let Some(stuff) = inbox.pop() {
                            if kill_switch.load(Ordering::Relaxed) {
                                manager.send(WorkerMessage::Slain).ok();
                                break;
                            }
                            let widgets = fun.improve(stuff);
                            let mut undone = Vec::with_capacity(widgets.len());
                            for widget in widgets {
                                if fun.inspect(&widget) {
                                    container.send(Some(widget)).ok();
                                } else {
                                    undone.push(widget);
                                }
                            }
                            if !undone.is_empty() {
                                let mut tithe =
                                    cmp::min(wanted.load(Ordering::Relaxed), undone.len() - 1);
                                if tithe > 0 {
                                    wanted.fetch_add(tithe, Ordering::Relaxed);
                                    let mut belt = belt.lock().unwrap();
                                    while tithe > 0 {
                                        belt.push(undone.pop().unwrap());
                                        tithe -= 1;
                                    }
                                    manager.send(WorkerMessage::WakeUp).ok();
                                }
                                inbox.extend(undone);
                            }
                        }
                    }
                    manager.send(WorkerMessage::Sleeping(i)).ok(); // send I'm empty message
                }
            })
        })
        .collect::<Vec<_>>();
    thread::spawn(move || {
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
    });
    (truck, kill_switch)
}
