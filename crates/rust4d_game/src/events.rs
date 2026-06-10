//! Typed event bus for game events
//!
//! A simple event bus that supports typed events with multiple handlers.
//! Events are queued with `emit()` and dispatched to handlers with `dispatch()`.
//!
//! ## Limitations
//!
//! Handlers cannot emit new events during dispatch. They receive `&dyn Any` with
//! no access to the `EventBus`, so follow-up events must be emitted by the caller
//! after `dispatch()` returns. This is intentional: the `EventBus` will be replaced
//! by Lua callbacks in Wave 3, so cascading event support is deferred.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Opaque identifier for a registered event handler
///
/// Returned by [`EventBus::on`] and used with [`EventBus::off`] to remove handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerId(TypeId, usize);

/// A single event handler function (must be thread-safe)
type EventHandler = Box<dyn Fn(&dyn Any) + Send + Sync>;

/// Map from event type to list of (id_counter, handler) pairs
type HandlerMap = HashMap<TypeId, Vec<(usize, EventHandler)>>;

/// A typed event bus for game events
///
/// Events are queued with [`emit`](EventBus::emit) and dispatched to registered
/// handlers with [`dispatch`](EventBus::dispatch). Each event type can have
/// multiple handlers, and events of different types are kept separate.
///
/// All handlers must be `Send + Sync` to allow the `EventBus` to be shared
/// across threads.
pub struct EventBus {
    /// Handlers indexed by event TypeId, each with a unique counter for removal
    handlers: HandlerMap,
    /// Queued events waiting to be dispatched
    queued: Vec<(TypeId, Box<dyn Any + Send + Sync>)>,
    /// Monotonically increasing counter for handler IDs
    next_id: usize,
}

impl EventBus {
    /// Create a new empty event bus
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            queued: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a handler for events of type T
    ///
    /// The handler will be called for each event of type T when `dispatch()` is called.
    /// Multiple handlers can be registered for the same event type.
    ///
    /// Returns a [`HandlerId`] that can be passed to [`off`](EventBus::off) to remove
    /// the handler.
    pub fn on<T: 'static>(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> HandlerId {
        let type_id = TypeId::of::<T>();
        let id = self.next_id;
        self.next_id += 1;

        let wrapper: EventHandler = Box::new(move |any| {
            if let Some(event) = any.downcast_ref::<T>() {
                handler(event);
            }
        });
        self.handlers
            .entry(type_id)
            .or_default()
            .push((id, wrapper));
        HandlerId(type_id, id)
    }

    /// Remove a previously registered handler
    ///
    /// Returns `true` if the handler was found and removed, `false` if not found
    /// (e.g., already removed or invalid ID).
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        let HandlerId(type_id, id) = handler_id;
        if let Some(handlers) = self.handlers.get_mut(&type_id) {
            let len_before = handlers.len();
            handlers.retain(|(h_id, _)| *h_id != id);
            handlers.len() < len_before
        } else {
            false
        }
    }

    /// Queue an event to be dispatched
    ///
    /// The event is stored and will be delivered to handlers when `dispatch()` is called.
    pub fn emit<T: 'static + Send + Sync>(&mut self, event: T) {
        let type_id = TypeId::of::<T>();
        self.queued.push((type_id, Box::new(event)));
    }

    /// Dispatch all queued events to their handlers
    ///
    /// Events are delivered in the order they were emitted. After dispatching,
    /// the event queue is cleared.
    ///
    /// **Note**: Handlers cannot emit new events during dispatch. Any follow-up
    /// events must be emitted by the caller after `dispatch()` returns.
    pub fn dispatch(&mut self) {
        // Drain the queue to avoid borrow conflicts
        let events: Vec<_> = self.queued.drain(..).collect();
        for (type_id, event) in &events {
            if let Some(handlers) = self.handlers.get(type_id) {
                for (_id, handler) in handlers {
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

    /// Returns the number of handlers registered for a given event type
    pub fn handler_count<T: 'static>(&self) -> usize {
        let type_id = TypeId::of::<T>();
        self.handlers.get(&type_id).map(|h| h.len()).unwrap_or(0)
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
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

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
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));

        let received_clone = received.clone();
        bus.on::<DamageEvent>(move |event| {
            received_clone.lock().unwrap().push(event.amount);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(DamageEvent { amount: 20 });

        assert_eq!(bus.queued_count(), 2);
        bus.dispatch();
        assert_eq!(bus.queued_count(), 0);

        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], 10);
        assert_eq!(received[1], 20);
    }

    #[test]
    fn test_multiple_handlers_for_same_type() {
        let mut bus = EventBus::new();
        let count1 = Arc::new(AtomicU32::new(0));
        let count2 = Arc::new(AtomicU32::new(0));

        let c1 = count1.clone();
        bus.on::<DamageEvent>(move |event| {
            c1.fetch_add(event.amount, Ordering::Relaxed);
        });

        let c2 = count2.clone();
        bus.on::<DamageEvent>(move |event| {
            c2.fetch_add(event.amount * 2, Ordering::Relaxed);
        });

        bus.emit(DamageEvent { amount: 5 });
        bus.dispatch();

        assert_eq!(count1.load(Ordering::Relaxed), 5);
        assert_eq!(count2.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_events_of_different_types_dont_cross() {
        let mut bus = EventBus::new();
        let damage_received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let heal_received = Arc::new(std::sync::Mutex::new(Vec::new()));

        let dr = damage_received.clone();
        bus.on::<DamageEvent>(move |event| {
            dr.lock().unwrap().push(event.amount);
        });

        let hr = heal_received.clone();
        bus.on::<HealEvent>(move |event| {
            hr.lock().unwrap().push(event.amount);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(HealEvent { amount: 25 });
        bus.emit(DamageEvent { amount: 5 });
        bus.dispatch();

        let damage = damage_received.lock().unwrap();
        assert_eq!(damage.len(), 2);
        assert_eq!(damage[0], 10);
        assert_eq!(damage[1], 5);

        let heal = heal_received.lock().unwrap();
        assert_eq!(heal.len(), 1);
        assert_eq!(heal[0], 25);
    }

    #[test]
    fn test_clear_removes_queued_events() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicU32::new(0));

        let c = count.clone();
        bus.on::<DamageEvent>(move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.emit(DamageEvent { amount: 20 });
        assert_eq!(bus.queued_count(), 2);

        bus.clear();
        assert_eq!(bus.queued_count(), 0);

        bus.dispatch();
        assert_eq!(
            count.load(Ordering::Relaxed),
            0,
            "No events should have been dispatched after clear"
        );
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
        let count = Arc::new(AtomicU32::new(0));

        let c = count.clone();
        bus.on::<DamageEvent>(move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch();
        assert_eq!(count.load(Ordering::Relaxed), 1);

        // Dispatching again should not re-deliver
        bus.dispatch();
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_default() {
        let bus = EventBus::default();
        assert_eq!(bus.queued_count(), 0);
    }

    #[test]
    fn test_on_returns_handler_id() {
        let mut bus = EventBus::new();
        let id1 = bus.on::<DamageEvent>(|_| {});
        let id2 = bus.on::<DamageEvent>(|_| {});

        // Each handler should get a unique ID
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_off_removes_handler() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicU32::new(0));

        let c = count.clone();
        let id = bus.on::<DamageEvent>(move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        assert_eq!(bus.handler_count::<DamageEvent>(), 1);

        // Remove the handler
        let removed = bus.off(id);
        assert!(removed, "Handler should be found and removed");
        assert_eq!(bus.handler_count::<DamageEvent>(), 0);

        // Events should not be delivered
        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch();
        assert_eq!(
            count.load(Ordering::Relaxed),
            0,
            "Removed handler should not be called"
        );
    }

    #[test]
    fn test_off_returns_false_for_unknown_id() {
        let mut bus = EventBus::new();
        let id = bus.on::<DamageEvent>(|_| {});

        // Remove once
        assert!(bus.off(id));

        // Second removal returns false
        assert!(!bus.off(id));
    }

    #[test]
    fn test_off_only_removes_target_handler() {
        let mut bus = EventBus::new();
        let count1 = Arc::new(AtomicU32::new(0));
        let count2 = Arc::new(AtomicU32::new(0));

        let c1 = count1.clone();
        let id1 = bus.on::<DamageEvent>(move |_| {
            c1.fetch_add(1, Ordering::Relaxed);
        });

        let c2 = count2.clone();
        let _id2 = bus.on::<DamageEvent>(move |_| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        // Remove only handler 1
        bus.off(id1);
        assert_eq!(bus.handler_count::<DamageEvent>(), 1);

        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch();

        assert_eq!(
            count1.load(Ordering::Relaxed),
            0,
            "Removed handler should not fire"
        );
        assert_eq!(
            count2.load(Ordering::Relaxed),
            1,
            "Remaining handler should fire"
        );
    }

    #[test]
    fn test_event_bus_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        // This fails to compile if EventBus is not Send+Sync
        assert_send_sync::<EventBus>();
    }

    /// Document that handlers cannot emit events during dispatch (I3/T3)
    #[test]
    fn test_handlers_cannot_emit_during_dispatch() {
        // Handlers receive &dyn Any with no access to the EventBus.
        // This means follow-up events must be emitted by the caller
        // after dispatch() returns.
        //
        // This is a known limitation. The EventBus will be replaced
        // by Lua callbacks in Wave 3, so cascading event support is deferred.

        let mut bus = EventBus::new();
        let was_called = Arc::new(AtomicU32::new(0));

        let wc = was_called.clone();
        bus.on::<DamageEvent>(move |_event| {
            // Cannot call bus.emit() here -- no access to the bus.
            // This test documents this intentional limitation.
            wc.fetch_add(1, Ordering::Relaxed);
        });

        bus.emit(DamageEvent { amount: 10 });
        bus.dispatch();

        assert_eq!(was_called.load(Ordering::Relaxed), 1);
    }
}
