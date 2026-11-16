//! The status bar's CPU reading. Stands in for `CpuUsage`, whose 64-bit time assembly is wrong: this computes what it meant, this
//! process's CPU time as a share of all the CPUs' time since the last reading.

use std::time::{Duration, Instant};

/// `EnoughTimePassed`.
const MIN_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
pub struct CpuUsage {
    last: Option<(f64, Instant)>,
    value: Option<i16>,
}

impl CpuUsage {
    pub fn new() -> CpuUsage {
        CpuUsage::default()
    }

    /// `None` until a second reading has something to compare with, or within 250 ms of the first. After that, a reading within
    /// 250 ms of the last repeats it.
    pub fn get_usage(&mut self) -> Option<i16> {
        let now = Instant::now();
        if let Some((_, at)) = self.last {
            if now.duration_since(at) < MIN_INTERVAL {
                return self.value;
            }
        }
        let cpu = process_cpu_seconds();
        if let Some((previous, at)) = self.last {
            let wall = now.duration_since(at).as_secs_f64();
            let cpus = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
            if wall > 0.0 {
                self.value = Some(((cpu - previous) / (wall * cpus) * 100.0) as i16);
            }
        }
        self.last = Some((cpu, now));
        self.value
    }
}

fn process_cpu_seconds() -> f64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: the pointer refers to a live value for the length of the call.
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let seconds = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1_000_000.0;
    seconds(usage.ru_utime) + seconds(usage.ru_stime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_until_there_is_something_to_compare() {
        let mut cpu = CpuUsage::new();
        assert_eq!(cpu.get_usage(), None);
        assert_eq!(cpu.get_usage(), None, "within 250 ms of the first");
    }

    #[test]
    fn a_busy_spin_reads_between_0_and_100_and_repeats_within_the_interval() {
        let mut cpu = CpuUsage::new();
        cpu.get_usage();
        let until = Instant::now() + Duration::from_millis(300);
        let mut x = 0u64;
        while Instant::now() < until {
            x = x.wrapping_add(1);
        }
        std::hint::black_box(x);
        let value = cpu.get_usage().expect("a reading");
        assert!((0..=100).contains(&value), "{value}");
        assert_eq!(cpu.get_usage(), Some(value));
    }
}
