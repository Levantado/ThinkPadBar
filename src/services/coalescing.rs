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

#[cfg(test)]
mod tests {
    use super::ValueCoalescer;

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
}
