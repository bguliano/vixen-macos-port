//! CPU copy of the light colours, and which parts of it the GPU buffer is missing.
//!
//! `set_light_colors` takes the whole array, so finding what changed is a scan, but only a memcmp-speed one,
//! and only what differs is uploaded. That is what makes a frame's upload follow the number of lights that
//! changed instead of the number of lights.

use std::ops::Range;

/// Lights compared and tracked together.
const BLOCK: usize = 64;
/// Blocks a gap can span and still be copied through: a copy command costs far more than 2 KiB of memcpy.
const MERGE_GAP_BLOCKS: usize = 8;
/// A frame issues at most this many copy commands, however scattered the changes are.
const MAX_SPANS: usize = 32;

#[derive(Default)]
pub(crate) struct ColorShadow {
    values: Vec<u32>,
    /// One bit per block.
    dirty: Vec<u64>,
    any_dirty: bool,
}

impl ColorShadow {
    #[cfg(test)]
    fn values(&self) -> &[u32] {
        &self.values
    }

    /// Replaces every colour. Entries past the end of `new` go transparent, so a short array can't leave
    /// stale lights on screen.
    pub(crate) fn set_all(&mut self, new: &[u32]) {
        if new.len() > self.values.len() {
            self.grow(new.len());
        }
        let (head, tail) = self.values.split_at_mut(new.len());

        let dirty = &mut self.dirty;
        let mut any_dirty = false;
        for (block, (dst, src)) in head.chunks_mut(BLOCK).zip(new.chunks(BLOCK)).enumerate() {
            if dst != src {
                dst.copy_from_slice(src);
                dirty[block / 64] |= 1 << (block % 64);
                any_dirty = true;
            }
        }
        let tail_start = new.len();
        for (offset, value) in tail.iter_mut().enumerate() {
            if *value != 0 {
                *value = 0;
                let block = (tail_start + offset) / BLOCK;
                dirty[block / 64] |= 1 << (block % 64);
                any_dirty = true;
            }
        }
        self.any_dirty |= any_dirty;
    }

    /// Makes room for `len` lights, keeping the colours already there. New lights start transparent.
    pub(crate) fn ensure_len(&mut self, len: usize) {
        if len > self.values.len() {
            self.grow(len);
        }
    }

    fn grow(&mut self, len: usize) {
        self.values.resize(len, 0);
        self.dirty.resize(len.div_ceil(BLOCK).div_ceil(64), 0);
    }

    /// Records that the GPU buffer doesn't hold `range` any more.
    pub(crate) fn mark(&mut self, range: Range<usize>) {
        let end = range.end.min(self.values.len());
        if range.start >= end {
            return;
        }
        for block in range.start / BLOCK..=(end - 1) / BLOCK {
            self.dirty[block / 64] |= 1 << (block % 64);
        }
        self.any_dirty = true;
    }

    /// Whether anything below `count`, the number of lights drawn, needs uploading.
    pub(crate) fn is_dirty_below(&self, count: usize) -> bool {
        self.any_dirty && self.first_dirty_block().is_some_and(|block| block * BLOCK < count)
    }

    fn first_dirty_block(&self) -> Option<usize> {
        self.dirty
            .iter()
            .position(|&word| word != 0)
            .map(|word| word * 64 + self.dirty[word].trailing_zeros() as usize)
    }

    /// Hands `upload` each run of colours to copy and its first light, clipped to `count`, and marks
    /// everything clean.
    ///
    /// Marks beyond `count` are dropped too: lights added later are marked when they are added.
    pub(crate) fn drain(&mut self, count: usize, mut upload: impl FnMut(usize, &[u32])) {
        if !self.any_dirty {
            return;
        }
        self.any_dirty = false;

        let values = &self.values;
        let mut flush = |span: Range<usize>| upload(span.start, &values[span]);

        let mut spans = 0;
        let mut current: Option<Range<usize>> = None;
        for (word_index, word) in self.dirty.iter_mut().enumerate() {
            let mut bits = std::mem::take(word);
            while bits != 0 {
                let block = word_index * 64 + bits.trailing_zeros() as usize;
                bits &= bits - 1;

                let start = block * BLOCK;
                let end = (start + BLOCK).min(count);
                if start >= count {
                    continue;
                }
                match &mut current {
                    // The last span allowed takes everything left, gaps included.
                    Some(span)
                        if start <= span.end + MERGE_GAP_BLOCKS * BLOCK
                            || spans + 1 == MAX_SPANS =>
                    {
                        span.end = end;
                    }
                    _ => {
                        if let Some(span) = current.take() {
                            flush(span);
                            spans += 1;
                        }
                        current = Some(start..end);
                    }
                }
            }
        }
        if let Some(span) = current {
            flush(span);
        }
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // A single span of lights is exactly what these expect.
mod tests {
    use super::*;

    fn drained(shadow: &mut ColorShadow, count: usize) -> Vec<Range<usize>> {
        let mut spans = Vec::new();
        shadow.drain(count, |start, values| spans.push(start..start + values.len()));
        spans
    }

    #[test]
    fn identical_colours_upload_nothing() {
        let mut shadow = ColorShadow::default();
        let colors = vec![7u32; 1000];
        shadow.set_all(&colors);
        assert_eq!(drained(&mut shadow, 1000), [0..1000]);
        shadow.set_all(&colors);
        assert!(!shadow.is_dirty_below(1000));
        assert!(drained(&mut shadow, 1000).is_empty());
    }

    #[test]
    fn a_single_change_uploads_one_block() {
        let mut shadow = ColorShadow::default();
        let mut colors = vec![1u32; 10_000];
        shadow.set_all(&colors);
        drained(&mut shadow, 10_000);

        colors[5000] = 2;
        shadow.set_all(&colors);
        let spans = drained(&mut shadow, 10_000);
        assert_eq!(spans.len(), 1);
        assert!(spans[0].contains(&5000));
        assert_eq!(spans[0].len(), BLOCK);
        assert_eq!(shadow.values()[5000], 2);
    }

    #[test]
    fn changes_from_two_sets_before_a_frame_both_upload_without_covering_the_gap() {
        let mut shadow = ColorShadow::default();
        let mut colors = vec![1u32; 100_000];
        shadow.set_all(&colors);
        drained(&mut shadow, 100_000);

        // The second set touches a lower index than the first, which a span list would have to merge.
        colors[90_000] = 2;
        shadow.set_all(&colors);
        colors[10] = 2;
        shadow.set_all(&colors);

        let spans = drained(&mut shadow, 100_000);
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert!(spans[0].contains(&10));
        assert!(spans[1].contains(&90_000));
    }

    #[test]
    fn nearby_changes_merge_and_scattered_changes_are_capped() {
        let mut shadow = ColorShadow::default();
        let mut colors = vec![1u32; 1_000_000];
        shadow.set_all(&colors);
        drained(&mut shadow, 1_000_000);

        colors[0] = 2;
        colors[BLOCK * 3] = 2;
        shadow.set_all(&colors);
        assert_eq!(drained(&mut shadow, 1_000_000).len(), 1);

        for i in 0..1000 {
            colors[i * 1000] += 1;
        }
        shadow.set_all(&colors);
        let spans = drained(&mut shadow, 1_000_000);
        assert_eq!(spans.len(), MAX_SPANS);
        // Disjoint, ascending, and nothing lost: the last change is inside the last span.
        assert!(spans.windows(2).all(|pair| pair[0].end < pair[1].start));
        assert!(spans.last().unwrap().contains(&999_000));
    }

    #[test]
    fn short_array_blanks_the_rest() {
        let mut shadow = ColorShadow::default();
        shadow.set_all(&vec![9u32; 500]);
        drained(&mut shadow, 500);

        shadow.set_all(&vec![9u32; 100]);
        let spans = drained(&mut shadow, 500);
        assert_eq!(spans.last().unwrap().end, 500);
        assert!(shadow.values()[100..].iter().all(|&c| c == 0));
        assert!(shadow.values()[..100].iter().all(|&c| c == 9));
    }

    #[test]
    fn clipped_to_the_drawn_count() {
        let mut shadow = ColorShadow::default();
        shadow.set_all(&vec![9u32; 1000]);
        assert!(!shadow.is_dirty_below(0));
        assert!(shadow.is_dirty_below(1));
        assert_eq!(drained(&mut shadow, 100), [0..100]);
        assert!(drained(&mut shadow, 1000).is_empty());
    }

    #[test]
    fn marking_handles_empty_and_out_of_range() {
        let mut shadow = ColorShadow::default();
        shadow.mark(0..10);
        assert!(!shadow.is_dirty_below(10));
        shadow.ensure_len(10);
        shadow.mark(5..5);
        assert!(!shadow.is_dirty_below(10));
        shadow.mark(5..1000);
        assert_eq!(drained(&mut shadow, 10), [0..10]);
    }
}
