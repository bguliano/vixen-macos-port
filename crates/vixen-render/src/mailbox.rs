//! Lock-free "latest frame" mailbox for light colours: a triple buffer.
//!
//! Three slots. The publisher owns one, the render thread owns one, and the third sits in the middle holding
//! the newest finished frame. Each side only ever swaps its own slot with the middle one, atomically, so
//! neither can make the other wait and the render thread always gets the newest complete frame.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

const INDEX_MASK: u8 = 0b011;
/// Set on `middle` while it holds a frame the render thread hasn't taken.
const FRESH: u8 = 0b100;

pub(crate) struct Mailbox {
    slots: [UnsafeCell<Vec<u32>>; 3],
    /// Index of the middle slot, plus `FRESH`.
    middle: AtomicU8,
    /// Index of the publisher's slot. Only touched while holding `publishing`.
    back: UnsafeCell<u8>,
    /// Keeps a second concurrent publisher out of the publisher's slot, without making it wait.
    publishing: AtomicBool,
    dropped: AtomicU64,
}

// SAFETY: a slot is only touched by whichever side holds its index, and the atomic swaps on `middle` hand an
// index over with the synchronisation that side's writes need. `back` is guarded by `publishing`.
unsafe impl Sync for Mailbox {}
// SAFETY: as above; nothing in here is tied to a thread.
unsafe impl Send for Mailbox {}

impl Mailbox {
    /// The reader is the only way to take frames, which is what makes its slot exclusively its own.
    pub(crate) fn new() -> (Arc<Mailbox>, MailboxReader) {
        let mailbox = Arc::new(Mailbox {
            slots: Default::default(),
            middle: AtomicU8::new(1),
            back: UnsafeCell::new(0),
            publishing: AtomicBool::new(false),
            dropped: AtomicU64::new(0),
        });
        let reader = MailboxReader { mailbox: mailbox.clone(), front: 2 };
        (mailbox, reader)
    }

    /// Copies `frame` in and makes it the newest. Never blocks: the copy needs no one else's cooperation, and
    /// if another thread is mid-publish this frame is counted as dropped.
    pub(crate) fn publish(&self, frame: &[u32]) {
        if self.publishing.swap(true, Ordering::Acquire) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // SAFETY: `publishing` gives this thread `back`, and `back` names a slot neither `middle` nor the
        // reader holds.
        unsafe {
            let back = *self.back.get();
            let slot = &mut *self.slots[back as usize].get();
            slot.clear();
            slot.extend_from_slice(frame);

            let previous = self.middle.swap(back | FRESH, Ordering::AcqRel);
            *self.back.get() = previous & INDEX_MASK;
            if previous & FRESH != 0 {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.publishing.store(false, Ordering::Release);
    }

    /// Frames replaced before the render thread took them.
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

pub(crate) struct MailboxReader {
    mailbox: Arc<Mailbox>,
    front: u8,
}

impl MailboxReader {
    /// The newest frame, if one arrived since the last call.
    pub(crate) fn take(&mut self) -> Option<&[u32]> {
        if self.mailbox.middle.load(Ordering::Acquire) & FRESH == 0 {
            return None;
        }
        let previous = self.mailbox.middle.swap(self.front, Ordering::AcqRel);
        self.front = previous & INDEX_MASK;
        // SAFETY: the swap handed this reader `front`, and it stays this reader's until the next `take`,
        // which the borrow on `self` prevents while the slice is alive.
        Some(unsafe { &*self.mailbox.slots[self.front as usize].get() })
    }

    /// Counts a frame the reader took but replaced before it was drawn.
    pub(crate) fn count_dropped(&self) {
        self.mailbox.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn takes_the_newest_frame_once() {
        let (mailbox, mut reader) = Mailbox::new();
        assert!(reader.take().is_none());

        mailbox.publish(&[1, 1]);
        mailbox.publish(&[2, 2, 2]);
        assert_eq!(reader.take(), Some(&[2u32, 2, 2][..]));
        assert!(reader.take().is_none());
        assert_eq!(mailbox.dropped(), 1);

        mailbox.publish(&[]);
        assert_eq!(reader.take(), Some(&[][..]));
    }

    #[test]
    fn a_stalled_reader_never_holds_up_the_publisher() {
        let (mailbox, mut reader) = Mailbox::new();
        mailbox.publish(&[1]);
        let held = reader.take().unwrap().to_vec();

        // The reader never comes back. Every publish still completes and the newest one wins.
        for i in 0..10_000 {
            mailbox.publish(&[i]);
        }
        assert_eq!(held, [1]);
        assert_eq!(reader.take(), Some(&[9999u32][..]));
        assert_eq!(mailbox.dropped(), 9999);
    }

    /// Every frame is filled with one value, so a torn frame shows up as two values in one slice. Values go
    /// up, so going backwards would show up too.
    #[test]
    fn frames_are_never_torn_or_reordered() {
        const LEN: usize = 4096;
        let (mailbox, mut reader) = Mailbox::new();
        let done = Arc::new(AtomicBool::new(false));

        let writer = {
            let (mailbox, done) = (mailbox.clone(), done.clone());
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_millis(300);
                let mut frame = vec![0u32; LEN];
                let mut value = 0;
                while Instant::now() < deadline {
                    value += 1;
                    frame.fill(value);
                    mailbox.publish(&frame);
                }
                done.store(true, Ordering::Release);
                value
            })
        };

        let mut last = 0;
        let mut taken = 0u64;
        loop {
            let finished = done.load(Ordering::Acquire);
            if let Some(frame) = reader.take() {
                assert_eq!(frame.len(), LEN);
                let value = frame[0];
                assert!(frame.iter().all(|&v| v == value), "torn frame");
                assert!(value > last, "frame went backwards: {value} after {last}");
                last = value;
                taken += 1;
            } else if finished {
                break;
            }
        }

        let published = writer.join().unwrap();
        assert_eq!(last, published, "the final frame must arrive");
        assert_eq!(taken + mailbox.dropped(), published as u64, "every frame is taken or counted as dropped");
    }

    #[test]
    fn concurrent_publishers_are_safe_and_never_wait() {
        const LEN: usize = 1024;
        let (mailbox, mut reader) = Mailbox::new();

        let writers: Vec<_> = (1..=4u32)
            .map(|id| {
                let mailbox = mailbox.clone();
                thread::spawn(move || {
                    let frame = vec![id; LEN];
                    for _ in 0..20_000 {
                        mailbox.publish(&frame);
                    }
                })
            })
            .collect();

        while writers.iter().any(|w| !w.is_finished()) {
            if let Some(frame) = reader.take() {
                let value = frame[0];
                assert!(frame.iter().all(|&v| v == value), "torn frame");
            }
        }
        for writer in writers {
            writer.join().unwrap();
        }
    }
}
