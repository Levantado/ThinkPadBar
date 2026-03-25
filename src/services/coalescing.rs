use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueCoalescer<T> {
    generation: u64,
    pending: Option<T>,
}

impl<T> Default for ValueCoalescer<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            pending: None,
        }
    }
}

impl<T> ValueCoalescer<T> {
    pub fn push(&mut self, value: T) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.pending = Some(value);
        self.generation
    }

    pub fn take_if_current(&mut self, generation: u64) -> Option<T> {
        if self.generation != generation {
            return None;
        }
        self.pending.take()
    }

    #[cfg(test)]
    fn pending(&self) -> Option<&T> {
        self.pending.as_ref()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RequestState {
    inflight: bool,
    queued: bool,
}

#[derive(Debug, Clone)]
pub struct RequestCoalescer<K> {
    states: HashMap<K, RequestState>,
}

impl<K> Default for RequestCoalescer<K> {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
        }
    }
}

impl<K> RequestCoalescer<K>
where
    K: Eq + Hash,
{
    pub fn request(&mut self, key: K) -> bool {
        let state = self.states.entry(key).or_default();
        if state.inflight {
            state.queued = true;
            return false;
        }

        state.inflight = true;
        state.queued = false;
        true
    }

    pub fn complete(&mut self, key: &K) -> bool {
        let Some(state) = self.states.get_mut(key) else {
            return false;
        };

        if state.queued {
            state.queued = false;
            state.inflight = true;
            return true;
        }

        self.states.remove(key);
        false
    }

    #[cfg(test)]
    pub fn is_inflight(&self, key: &K) -> bool {
        self.states.get(key).is_some_and(|state| state.inflight)
    }

    #[cfg(test)]
    pub fn is_queued(&self, key: &K) -> bool {
        self.states.get(key).is_some_and(|state| state.queued)
    }
}

#[cfg(test)]
mod tests {
    use super::{RequestCoalescer, ValueCoalescer};

    #[test]
    fn stale_generation_does_not_consume_latest_value() {
        let mut coalescer = ValueCoalescer::default();
        let first = coalescer.push(10_u32);
        let second = coalescer.push(20_u32);

        assert_eq!(coalescer.take_if_current(first), None);
        assert_eq!(coalescer.pending(), Some(&20));
        assert_eq!(coalescer.take_if_current(second), Some(20));
        assert_eq!(coalescer.pending(), None);
    }

    #[test]
    fn current_generation_consumes_only_once() {
        let mut coalescer = ValueCoalescer::default();
        let generation = coalescer.push("latest");

        assert_eq!(coalescer.take_if_current(generation), Some("latest"));
        assert_eq!(coalescer.take_if_current(generation), None);
    }

    #[test]
    fn request_coalescer_restarts_once_after_inflight_completion() {
        let mut coalescer = RequestCoalescer::default();

        assert!(coalescer.request("brightness"));
        assert!(!coalescer.request("brightness"));
        assert!(coalescer.is_inflight(&"brightness"));
        assert!(coalescer.is_queued(&"brightness"));

        assert!(coalescer.complete(&"brightness"));
        assert!(coalescer.is_inflight(&"brightness"));
        assert!(!coalescer.is_queued(&"brightness"));

        assert!(!coalescer.complete(&"brightness"));
        assert!(!coalescer.is_inflight(&"brightness"));
        assert!(!coalescer.is_queued(&"brightness"));
    }

    #[test]
    fn request_coalescer_tracks_distinct_keys_independently() {
        let mut coalescer = RequestCoalescer::default();

        assert!(coalescer.request("brightness"));
        assert!(coalescer.request("fan"));
        assert!(!coalescer.request("brightness"));

        assert!(coalescer.is_inflight(&"brightness"));
        assert!(coalescer.is_queued(&"brightness"));
        assert!(coalescer.is_inflight(&"fan"));
        assert!(!coalescer.is_queued(&"fan"));
    }
}
