use crate::*;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::random;

use std::cell::{Cell, RefCell};

pub type EventRegistry = Vec<Box<TimedEvent>>;

lazy_static! {
    static ref TIMED_EVENTS: Mutex<EventRegistry> = Mutex::new(Vec::new());
}

pub fn update_timed_events() {
    let mut registry = TIMED_EVENTS.lock();

    let events: EventRegistry = registry
        .drain_filter(|e| game_time() >= e.min_exe_time())
        .collect();

    for event in events {
        event.run();
        event.handle_delete(&mut *registry);
    }
}

pub fn delete_event(id: usize) -> Option<Box<TimedEvent>> {
    _delete_event(id, &mut *TIMED_EVENTS.lock())
}

fn _delete_event(id: usize, registry: &mut EventRegistry) -> Option<Box<TimedEvent>> {
    registry
        .iter()
        .position(|e| e.matches_id(id))
        .and_then(|i| Some(registry.remove(i)))
}

/**
 * Not super efficient going through the entire array for
 * every single match + 1. Them's the borrow rules, though.
 */
pub fn delete_by_flags(
    area: Option<usize>,
    entity: Option<usize>,
    flag: Option<&str>,
) -> Vec<Box<TimedEvent>> {
    TIMED_EVENTS
        .lock()
        .drain_filter(|e| {
            let mut condition = true;
            area.and_then(|a| Some(condition &= e.matches_area(a)));
            entity.and_then(|ent| Some(condition &= e.matches_entity(ent)));
            flag.and_then(|f| Some(condition &= e.matches_flag(f)));
            condition
        })
        .collect()
}

fn schedule_event(event: impl TimedEvent + 'static) {
    TIMED_EVENTS.lock().push(Box::new(event));
}

fn get_exe_time(from_delay: u64) -> u64 {
    game_time() + from_delay
}

pub trait TimedEvent: Send {
    fn min_exe_time(&self) -> u64;

    fn run(&self);

    fn handle_delete(self: Box<Self>, registry: &mut EventRegistry);

    fn matches_area(&self, _area: usize) -> bool {
        true
    }

    fn matches_entity(&self, _entity: usize) -> bool {
        true
    }

    fn matches_flag(&self, _flag: &str) -> bool {
        true
    }

    fn matches_id(&self, id: usize) -> bool;
}

pub struct DelayedEvent<F: FnOnce() + Send> {
    exe_time: u64,
    run: RefCell<Option<F>>,
    area_id: Option<usize>,
    entity_id: Option<usize>,
    flag: Option<String>,
    id: usize,
}

impl<F: FnOnce() + 'static + Send> DelayedEvent<F> {
    pub fn no_flags(delay_ms: u64, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id,
        });
        id
    }

    pub fn all_flags(
        delay_ms: u64,
        area: usize,
        entity: usize,
        flag: String,
        callback: F,
    ) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: Some(area),
            entity_id: Some(entity),
            flag: Some(flag),
            id,
        });
        id
    }

    pub fn new_for_area(delay_ms: u64, area: usize, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: Some(area),
            entity_id: None,
            flag: None,
            id,
        });
        id
    }

    pub fn new_for_entity(delay_ms: u64, entity: usize, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: Some(entity),
            flag: None,
            id,
        });
        id
    }

    pub fn new_for_flag(delay_ms: u64, flag: &str, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: Some(flag.to_string()),
            id,
        });
        id
    }

    pub fn new(
        delay_ms: u64,
        area: Option<usize>,
        entity: Option<usize>,
        flag: Option<String>,
        callback: F,
    ) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: area,
            entity_id: entity,
            flag,
            id,
        });
        id
    }
}

#[derive(Clone)]
pub struct DelayHandler {
    exe_time: u64,
}

impl DelayHandler {
    pub fn new(delay_ms: u64) -> DelayHandler {
        DelayHandler {
            exe_time: get_exe_time(delay_ms),
        }
    }

    pub fn then<F: FnOnce() + 'static + Send>(&self, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: self.exe_time,
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id,
        });
        id
    }

    pub fn then_after<F: FnOnce() + 'static + Send>(&self, delay_ms: u64, callback: F) -> usize {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: self.exe_time + delay_ms,
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id,
        });
        id
    }
}

impl<F: FnOnce() + Send> TimedEvent for DelayedEvent<F> {
    fn min_exe_time(&self) -> u64 {
        self.exe_time
    }

    fn run(&self) {
        let old_fn = self.run.replace(None);

        if let Some(fun) = old_fn {
            (fun)();
        }
    }

    fn handle_delete(self: Box<Self>, _registry: &mut EventRegistry) {}

    fn matches_area(&self, area: usize) -> bool {
        if let Some(a) = self.area_id {
            a == area
        } else {
            false
        }
    }

    fn matches_entity(&self, entity: usize) -> bool {
        if let Some(e) = self.entity_id {
            e == entity
        } else {
            false
        }
    }

    fn matches_flag(&self, flag: &str) -> bool {
        if let Some(ref f) = self.flag {
            f == flag
        } else {
            false
        }
    }

    fn matches_id(&self, id: usize) -> bool {
        self.id == id
    }
}

impl<F: Fn() + Send> PartialEq for DelayedEvent<F> {
    fn eq(&self, other: &DelayedEvent<F>) -> bool {
        self.id == other.id
    }
}

/**
 * Accepts a callback which returns a boolean.
 * false -> early stop.
 */
pub struct RepeatedEvent<F: Fn() -> bool + Send> {
    next_exe_time: Cell<u64>,
    interval: u64,
    max_exe_time: u64,
    run: F,
    area_id: Option<usize>,
    entity_id: Option<usize>,
    flag: Option<String>,
    id: usize,
}

impl<F: Fn() -> bool + 'static + Send> RepeatedEvent<F> {
    pub fn no_flags(interval: u64, duration: u64, callback: F) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: None,
            flag: None,
            id,
        });
        id
    }

    pub fn all_flags(
        interval: u64,
        duration: u64,
        area: usize,
        entity: usize,
        flag: String,
        callback: F,
    ) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: Some(area),
            entity_id: Some(entity),
            flag: Some(flag),
            id,
        });
        id
    }

    pub fn new_for_area(interval: u64, duration: u64, area: usize, callback: F) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: Some(area),
            entity_id: None,
            flag: None,
            id,
        });
        id
    }

    pub fn new_for_entity(interval: u64, duration: u64, entity: usize, callback: F) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: Some(entity),
            flag: None,
            id,
        });
        id
    }

    pub fn new_for_flag(interval: u64, duration: u64, flag: &str, callback: F) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: None,
            flag: Some(flag.to_string()),
            id,
        });
        id
    }

    pub fn new(
        interval: u64,
        duration: u64,
        area: Option<usize>,
        entity: Option<usize>,
        flag: Option<String>,
        callback: F,
    ) -> usize {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: area,
            entity_id: entity,
            flag,
            id,
        });
        id
    }
}

impl<F: Fn() -> bool + 'static + Send> TimedEvent for RepeatedEvent<F> {
    fn min_exe_time(&self) -> u64 {
        self.next_exe_time.get()
    }

    fn run(&self) {
        if (&self.run)() {
            self.next_exe_time.set(get_exe_time(self.interval));
        } else {
            delete_event(self.id);
        }
    }

    fn handle_delete(self: Box<Self>, registry: &mut EventRegistry) {
        if game_time() <= self.max_exe_time {
            registry.push(self)
        }
    }

    fn matches_area(&self, area: usize) -> bool {
        if let Some(a) = self.area_id {
            a == area
        } else {
            false
        }
    }

    fn matches_entity(&self, entity: usize) -> bool {
        if let Some(e) = self.entity_id {
            e == entity
        } else {
            false
        }
    }

    fn matches_flag(&self, flag: &str) -> bool {
        if let Some(ref f) = self.flag {
            f == flag
        } else {
            false
        }
    }

    fn matches_id(&self, id: usize) -> bool {
        self.id == id
    }
}
