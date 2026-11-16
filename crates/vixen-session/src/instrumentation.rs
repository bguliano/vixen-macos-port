//! The values the Instrumentation window lists. Port of `Vixen.Core/Sys/Instrumentation` and `Vixen.Core/Instrumentation`.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// `IInstrumentationValue`.
pub trait InstrumentationValue: Send + Sync {
    fn name(&self) -> &str;
    fn formatted(&self) -> String;
    fn reset(&self);
}

/// `MillisecondsValue._GetFormattedValue`, keeping its quirks: `cnt` starts at 1 and `min` at 999, so a fresh value reads
/// `min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1`.
pub fn format_milliseconds(min: f64, total: f64, max: f64, count: i64) -> String {
    let average = if count == 0 { 0 } else { total as i64 / count };
    format!("min {} ms,  avg {} ms,  max {} ms,  cnt {}", min as i64, average, max as i64, count)
}

/// `MillisecondsValue`. Set from a frame thread and read from the UI, so it takes no lock.
pub struct MillisecondsValue {
    name: String,
    total: AtomicU64,
    count: AtomicI64,
    max: AtomicU64,
    min: AtomicU64,
}

impl MillisecondsValue {
    pub fn new(name: impl Into<String>) -> MillisecondsValue {
        let value = MillisecondsValue {
            name: name.into(),
            total: AtomicU64::new(0),
            count: AtomicI64::new(0),
            max: AtomicU64::new(0),
            min: AtomicU64::new(0),
        };
        value.reset();
        value
    }

    pub fn set(&self, value_ms: f64) {
        let update = |slot: &AtomicU64, f: &dyn Fn(f64) -> f64| {
            let _ = slot.try_update(Ordering::Relaxed, Ordering::Relaxed, |bits| Some(f(f64::from_bits(bits)).to_bits()));
        };
        update(&self.max, &|old| old.max(value_ms));
        update(&self.min, &|old| old.min(value_ms));
        update(&self.total, &|old| old + value_ms);
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}

impl InstrumentationValue for MillisecondsValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn formatted(&self) -> String {
        format_milliseconds(
            f64::from_bits(self.min.load(Ordering::Relaxed)),
            f64::from_bits(self.total.load(Ordering::Relaxed)),
            f64::from_bits(self.max.load(Ordering::Relaxed)),
            self.count.load(Ordering::Relaxed),
        )
    }

    fn reset(&self) {
        self.total.store(0f64.to_bits(), Ordering::Relaxed);
        self.count.store(1, Ordering::Relaxed);
        self.max.store(0f64.to_bits(), Ordering::Relaxed);
        self.min.store(999f64.to_bits(), Ordering::Relaxed);
    }
}

/// `RateValue` with `OutputDeviceRefreshRateValue`'s format. Reading resets the count, as `_GetValue` does.
pub struct RateValue {
    name: String,
    count: AtomicU64,
    since: Mutex<Instant>,
}

impl RateValue {
    pub fn new(name: impl Into<String>) -> RateValue {
        RateValue { name: name.into(), count: AtomicU64::new(0), since: Mutex::new(Instant::now()) }
    }

    pub fn increment(&self, by: u64) {
        self.count.fetch_add(by, Ordering::Relaxed);
    }

    fn rate(&self) -> f64 {
        let mut since = self.since.lock();
        let elapsed_ms = since.elapsed().as_millis() as f64;
        *since = Instant::now();
        let count = self.count.swap(0, Ordering::Relaxed) as f64;
        if elapsed_ms == 0.0 {
            return 0.0;
        }
        count * (1000.0 / elapsed_ms)
    }
}

impl InstrumentationValue for RateValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn formatted(&self) -> String {
        format!("{:.2} Hz", self.rate())
    }

    fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
    }
}

/// `Instrumentation`: values in the order they were first added. Adding a name again replaces the value and keeps the slot.
#[derive(Default)]
pub struct Instrumentation {
    values: Mutex<Vec<Arc<dyn InstrumentationValue>>>,
}

impl Instrumentation {
    pub fn new() -> Instrumentation {
        Instrumentation::default()
    }

    pub fn add(&self, value: Arc<dyn InstrumentationValue>) {
        let mut values = self.values.lock();
        match values.iter_mut().find(|v| v.name() == value.name()) {
            Some(slot) => *slot = value,
            None => values.push(value),
        }
    }

    pub fn remove(&self, name: &str) {
        self.values.lock().retain(|v| v.name() != name);
    }

    /// `{name}: {formatted}`, as the window's text box lists them.
    pub fn lines(&self) -> Vec<String> {
        self.values.lock().iter().map(|v| format!("{}: {}", v.name(), v.formatted())).collect()
    }

    pub fn reset_all(&self) {
        for value in self.values.lock().iter() {
            value.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_value_has_the_originals_odd_starting_numbers() {
        assert_eq!(MillisecondsValue::new("x").formatted(), "min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1");
    }

    #[test]
    fn setting_counts_from_one_and_averages_over_that() {
        let value = MillisecondsValue::new("x");
        value.set(10.0);
        value.set(20.0);
        assert_eq!(value.formatted(), "min 10 ms,  avg 10 ms,  max 20 ms,  cnt 3");
    }

    #[test]
    fn reset_returns_to_fresh() {
        let value = MillisecondsValue::new("x");
        value.set(10.0);
        value.reset();
        assert_eq!(value.formatted(), "min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1");
    }

    #[test]
    fn a_rate_has_two_decimals_and_a_second_read_is_zero() {
        let rate = RateValue::new("Dev refresh rate");
        rate.increment(5);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let first = rate.formatted();
        assert!(first.ends_with(" Hz"), "{first}");
        let number: f64 = first.trim_end_matches(" Hz").parse().unwrap();
        assert!(number > 0.0, "{first}");
        assert_eq!(first.split('.').nth(1).map(|d| d.len()), Some(5), "{first}");
        assert_eq!(rate.formatted(), "0.00 Hz");
    }

    #[test]
    fn the_list_keeps_its_order_and_a_second_add_keeps_the_slot() {
        let list = Instrumentation::new();
        let first = Arc::new(MillisecondsValue::new("A"));
        first.set(1.0);
        list.add(first);
        list.add(Arc::new(MillisecondsValue::new("B")));
        list.add(Arc::new(MillisecondsValue::new("A")));
        assert_eq!(
            list.lines(),
            ["A: min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1", "B: min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1"]
        );
        list.remove("A");
        assert_eq!(list.lines().len(), 1);
        assert!(list.lines()[0].starts_with("B: "));
    }

    #[test]
    fn reset_all_resets_every_value() {
        let list = Instrumentation::new();
        let value = Arc::new(MillisecondsValue::new("A"));
        value.set(5.0);
        list.add(value.clone());
        list.reset_all();
        assert_eq!(value.formatted(), "min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1");
    }
}
