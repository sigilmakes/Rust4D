//! Typed event bus for game events
//!
//! A simple event bus that supports typed events with multiple handlers.
//! Events are queued with `emit()` and dispatched to handlers with `dispatch()`.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A typed event bus for game events
///
/// Events are queued with [`emit`](EventBus::emit) and dispatched to registered
/// handlers with [`dispatch`](EventBus::dispatch). Each event type can have
/// multiple handlers, and events of different types are kept separate.
pub struct EventBus {
    /// Handlers indexed by event TypeId
    handlers: HashMap<TypeId, Vec<Box<dyn Fn(&dyn Any)>>>,
    /// Queued events waiting to be dispatched
    queued: Vec<(TypeId, Box<dyn Any>)>,
}

impl EventBus {
    /// Create a new empty event bus
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            queued: Vec::new(),
        }
    }

    /// Register a handler for events of type T
    ///
    /// The handler will be called for each event of type T when `dispatch()` is called.
    /// Multiple handlers can be registered for the same event type.
    pub fn on<T: 'static>(&mut self, handler: impl Fn(&T) + 'static) {
        let type_id = TypeId::of::<T>();
        let wrapper: Box<dyn Fn(&dyn Any)> = Box::new(move |any| {
            if let Some(event) = any.downcast_ref::<T>() {
                handler(event);
            }
        });
        self.handlers.entry(type_id).or_default().push(wrapper);
    }

    /// Queue an event to be dispatched
    ///
    /// The event is stored and will be delivered to handlers when `dispatch()` is called.
    pub fn emit<T: 'static>(&mut self, event: T) {
        let type_id = TypeId::of::<T>();
        self.queued.push((type_id, Box::new(event)));
    }

    /// Dispatch all queued events to their handlers
    ///
    /// Events are delivered in the order they were emitted. After dispatching,
    /// the event queue is cleared.
    pub fn dispatch(&mut self) {
        // Drain the queue to avoid borrow conflicts
        let events: Vec<_> = self.queued.drain(..).collect();
        for (type_id, event) in &events {
            if let Some(handlers) = self.handlers.get(type_id) {
                for handler in handlers {
                    handler(event.as_ref());
                }
            }
        }
    }

    /// Clear all queued events without dispatching
    pub fn clear(&mut self) {
        self.queued.clear();
    }

    /// Returns the number of queued events
    pub fn queued_count(&self) -> usize {
        self.queued.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Test event types
    #[derive(Debug, Clone)]
    struct DamageEvent {
        amount: u32,
    }

    #[derive(Debug, Clone)]
    struct HealEvent {
        amount: u32,
    }

    #[test]
    fn test_emit_and_dispatch_delivers_events() {
        let mut bus = EventBus::new();
        let received = Rc::new(RefCell::new(Vec::new()));

        let received_clone = received.clone();
        bus.on::<DamageEvent>(move |event| {
            received_clone.borrow_mut().push(event.amount);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(DamageEvent { amount: 20 });

        assert_eq!(bus.queued_count(), 2);
        bus.dispatch();
        assert_eq!(bus.queued_count(), 0);

        let received = received.borrow();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], 10);
        assert_eq!(received[1], 20);
    }

    #[test]
    fn test_multiple_handlers_for_same_type() {
        let mut bus = EventBus::new();
        let count1 = Rc::new(RefCell::new(0u32));
        let count2 = Rc::new(RefCell::new(0u32));

        let c1 = count1.clone();
        bus.on::<DamageEvent>(move |event| {
            *c1.borrow_mut() += event.amount;
        });

        let c2 = count2.clone();
        bus.on::<DamageEvent>(move |event| {
            *c2.borrow_mut() += event.amount * 2;
        });

        bus.emit(DamageEvent { amount: 5 });
        bus.dispatch();

        assert_eq!(*count1.borrow(), 5);
        assert_eq!(*count2.borrow(), 10);
    }

    #[test]
    fn test_events_of_different_types_dont_cross() {
        let mut bus = EventBus::new();
        let damage_received = Rc::new(RefCell::new(Vec::new()));
        let heal_received = Rc::new(RefCell::new(Vec::new()));

        let dr = damage_received.clone();
        bus.on::<DamageEvent>(move |event| {
            dr.borrow_mut().push(event.amount);
        });

        let hr = heal_received.clone();
        bus.on::<HealEvent>(move |event| {
            hr.borrow_mut().push(event.amount);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(HealEvent { amount: 25 });
        bus.emit(DamageEvent { amount: 5 });
        bus.dispatch();

        let damage = damage_received.borrow();
        assert_eq!(damage.len(), 2);
        assert_eq!(damage[0], 10);
        assert_eq!(damage[1], 5);

        let heal = heal_received.borrow();
        assert_eq!(heal.len(), 1);
        assert_eq!(heal[0], 25);
    }

    #[test]
    fn test_clear_removes_queued_events() {
        let mut bus = EventBus::new();
        let count = Rc::new(RefCell::new(0u32));

        let c = count.clone();
        bus.on::<DamageEvent>(move |_| {
            *c.borrow_mut() += 1;
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(DamageEvent { amount: 20 });
        assert_eq!(bus.queued_count(), 2);

        bus.clear();
        assert_eq!(bus.queued_count(), 0);

        bus.dispatch();
        assert_eq!(*count.borrow(), 0, "No events should have been dispatched after clear");
    }

    #[test]
    fn test_dispatch_without_handlers() {
        let mut bus = EventBus::new();

        // Emitting without any handlers should not panic
        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch(); // Should be a no-op
    }

    #[test]
    fn test_dispatch_clears_queue() {
        let mut bus = EventBus::new();
        let count = Rc::new(RefCell::new(0u32));

        let c = count.clone();
        bus.on::<DamageEvent>(move |_| {
            *c.borrow_mut() += 1;
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch();
        assert_eq!(*count.borrow(), 1);

        // Dispatching again should not re-deliver
        bus.dispatch();
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn test_default() {
        let bus = EventBus::default();
        assert_eq!(bus.queued_count(), 0);
    }
}
