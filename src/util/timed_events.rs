extern crate rand;

use rand::random;

use std::cell::{Cell, RefCell};

/**
 * Optimizations very much needed.
 */

static mut TIMED_EVENTS: Option<Vec<Box<TimedEvent>>> = None;

pub unsafe fn setup_event_registry()
{
    TIMED_EVENTS = Some(Vec::new());
}

pub fn update_timed_events()
{
    unsafe { if let Some(ref mut registry) = TIMED_EVENTS
    {
        for event in registry.iter()
        {
            if ::game_time() >= event.min_exe_time()
            {
                event.run();
                event.delete();
            }
        }
    }}
}

pub fn delete_event(id: usize) -> Option<Box<TimedEvent>>
{
    let mut ret = None;

    unsafe { if let Some(ref mut registry) = TIMED_EVENTS
    {
        let index = registry.iter().position(| e |
        {
            e.matches_id(id)
        });

        if let Some(num) = index
        {
            ret = Some(registry.remove(num));
        }
    }}
    ret
}

/**
 * Not super efficient going through the entire array for
 * every single match + 1. Them's the borrow rules, though.
 */
pub fn delete_by_flags(area: Option<usize>, entity: Option<usize>, flag: Option<&str>)
{
    unsafe { if let Some(ref mut registry) = TIMED_EVENTS
    {
        while let Some(num) = registry.iter().position(| e |
        {
            let mut ret = true;

            if let Some(a) = area
            {
                ret &= e.matches_area(a);
            }

            if let Some(ent) = entity
            {
                ret &= e.matches_entity(ent);
            }

            if let Some(f) = flag
            {
                ret &= e.matches_flag(f)
            }

            ret
        }){
            registry.remove(num);

            println!("Deleting event at index #{}.", num);
        }
    }}
}

fn schedule_event(event: impl TimedEvent + 'static)
{
    unsafe { if let Some(ref mut registry) = TIMED_EVENTS
    {
        registry.push(Box::new(event));
    }}
}

fn get_exe_time(from_delay: u64) -> u128
{
    ::game_time() + from_delay as u128
}

pub trait TimedEvent
{
    fn min_exe_time(&self) -> u128;

    fn run(&self);

    fn delete(&self);

    fn matches_area(&self, _area: usize) -> bool { true }

    fn matches_entity(&self, _entity: usize) -> bool { true }

    fn matches_flag(&self, _flag: &str) -> bool { true }

    fn matches_id(&self, id: usize) -> bool;
}

pub struct DelayedEvent<F: FnOnce()>
{
    exe_time: u128,
    run: RefCell<Option<F>>,
    area_id: Option<usize>,
    entity_id: Option<usize>,
    flag: Option<String>,
    id: usize
}

impl<F: FnOnce() + 'static> DelayedEvent<F>
{
    pub fn no_flags(delay_ms: u64, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id
        });
        id
    }

    pub fn all_flags(delay_ms: u64, area: usize, entity: usize, flag: String, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: Some(area),
            entity_id: Some(entity),
            flag: Some(flag),
            id
        });
        id
    }

    pub fn new_for_area(delay_ms: u64, area: usize, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: Some(area),
            entity_id: None,
            flag: None,
            id
        });
        id
    }

    pub fn new_for_entity(delay_ms: u64, entity: usize, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: Some(entity),
            flag: None,
            id
        });
        id
    }

    pub fn new_for_flag(delay_ms: u64, flag: &str, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: Some(flag.to_string()),
            id
        });
        id
    }

    pub fn new(delay_ms: u64, area: Option<usize>, entity: Option<usize>, flag: Option<String>, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent {
            exe_time: get_exe_time(delay_ms),
            run: RefCell::new(Some(callback)),
            area_id: area,
            entity_id: entity,
            flag,
            id
        });
        id
    }
}

#[derive(Clone)]
pub struct DelayHandler
{
    exe_time: u128
}

impl DelayHandler
{
    pub fn new(delay_ms: u64) -> DelayHandler
    {
        DelayHandler
        {
            exe_time: get_exe_time(delay_ms)
        }
    }

    pub fn then<F: FnOnce() + 'static>(&self, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent
        {
            exe_time: self.exe_time,
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id
        });
        id
    }

    pub fn then_after<F: FnOnce() + 'static>(&self, delay_ms: u64, callback: F) -> usize
    {
        let id = random();

        schedule_event(DelayedEvent
        {
            exe_time: self.exe_time + delay_ms as u128,
            run: RefCell::new(Some(callback)),
            area_id: None,
            entity_id: None,
            flag: None,
            id
        });
        id
    }
}

impl<F: FnOnce()> TimedEvent for DelayedEvent<F>
{
    fn min_exe_time(&self) -> u128
    {
        self.exe_time
    }

    fn run(&self)
    {
        let old_fn = self.run.replace(None);

        if let Some(fun) = old_fn
        {
            (fun)();
        }
    }

    fn delete(&self)
    {
        delete_event(self.id);
    }

    fn matches_area(&self, area: usize) -> bool
    {
        if let Some(a) = self.area_id
        {
            a == area
        }
        else { false }
    }

    fn matches_entity(&self, entity: usize) -> bool
    {
        if let Some(e) = self.entity_id
        {
            e == entity
        }
        else { false }
    }

    fn matches_flag(&self, flag: &str) -> bool
    {
        if let Some(ref f) = self.flag
        {
            f == flag
        }
        else { false }
    }

    fn matches_id(&self, id: usize) -> bool
    {
        self.id == id
    }
}

impl<F: Fn()> PartialEq for DelayedEvent<F>
{
    fn eq(&self, other: &DelayedEvent<F>) -> bool
    {
        self.id == other.id
    }
}

/**
 * Accepts a callback which returns a boolean.
 * false -> early stop.
 */
pub struct RepeatedEvent<F: Fn() -> bool>
{
    next_exe_time: Cell<u128>,
    interval: u64,
    max_exe_time: u128,
    run: F,
    area_id: Option<usize>,
    entity_id: Option<usize>,
    flag: Option<String>,
    id: usize
}

impl<F: Fn() -> bool + 'static> RepeatedEvent<F>
{
    pub fn no_flags(interval: u64, duration: u64, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: None,
            flag: None,
            id
        });
        id
    }

    pub fn all_flags(interval: u64, duration: u64, area: usize, entity: usize, flag: String, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: Some(area),
            entity_id: Some(entity),
            flag: Some(flag),
            id
        });
        id
    }

    pub fn new_for_area(interval: u64, duration: u64, area: usize, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: Some(area),
            entity_id: None,
            flag: None,
            id
        });
        id
    }

    pub fn new_for_entity(interval: u64, duration: u64, entity: usize, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: Some(entity),
            flag: None,
            id
        });
        id
    }

    pub fn new_for_flag(interval: u64, duration: u64, flag: &str, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: None,
            entity_id: None,
            flag: Some(flag.to_string()),
            id
        });
        id
    }

    pub fn new(interval: u64, duration: u64, area: Option<usize>, entity: Option<usize>, flag: Option<String>, callback: F) -> usize
    {
        let id = random();

        schedule_event(RepeatedEvent {
            next_exe_time: Cell::new(get_exe_time(interval)),
            interval,
            max_exe_time: get_exe_time(duration),
            run: callback,
            area_id: area,
            entity_id: entity,
            flag,
            id
        });
        id
    }
}

impl<F: Fn() -> bool> TimedEvent for RepeatedEvent<F>
{
    fn min_exe_time(&self) -> u128
    {
        self.next_exe_time.get()
    }

    fn run(&self)
    {
        if (&self.run)()
        {
            self.next_exe_time.set(get_exe_time(self.interval));
        }
        else { delete_event(self.id); }
    }

    fn delete(&self)
    {
        if ::game_time() >= self.max_exe_time
        {
            delete_event(self.id);
        }
    }

    fn matches_area(&self, area: usize) -> bool
    {
        if let Some(a) = self.area_id
        {
            a == area
        }
        else { false }
    }

    fn matches_entity(&self, entity: usize) -> bool
    {
        if let Some(e) = self.entity_id
        {
            e == entity
        }
        else { false }
    }

    fn matches_flag(&self, flag: &str) -> bool
    {
        if let Some(ref f) = self.flag
        {
            f == flag
        }
        else { false }
    }

    fn matches_id(&self, id: usize) -> bool
    {
        self.id == id
    }
}