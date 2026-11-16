/*
    Vamp

    An API for audio analysis and feature extraction plugins.

    Centre for Digital Music, Queen Mary, University of London.
    Copyright 2006 Chris Cannam.

    Permission is hereby granted, free of charge, to any person
    obtaining a copy of this software and associated documentation
    files (the "Software"), to deal in the Software without
    restriction, including without limitation the rights to use, copy,
    modify, merge, publish, distribute, sublicense, and/or sell copies
    of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be
    included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
    EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
    MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
    NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR
    ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
    CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
    WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

    Except as contained in this notice, the names of the Centre for
    Digital Music; Queen Mary, University of London; and Chris Cannam
    shall not be used in advertising or otherwise to promote the sale,
    use or other dealings in this Software without prior written
    authorization.
*/

/*
   This is a modified version of a source file from the
   Rosegarden MIDI and audio sequencer and notation editor.
   This file copyright 2000-2006 Chris Cannam.
   Relicensed by the author as detailed above.
*/

//! Port of the Vamp SDK types the tracker uses: `RealTime` (with `ManagedRealtime::totalMilliseconds`), `Feature`,
//! and the descriptors.

use std::collections::BTreeMap;
use std::ops::Add;

const ONE_BILLION: i32 = 1_000_000_000;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct RealTime {
    pub sec: i32,
    pub nsec: i32,
}

impl RealTime {
    pub const ZERO: RealTime = RealTime { sec: 0, nsec: 0 };

    pub fn new(s: i32, n: i32) -> RealTime {
        let (mut sec, mut nsec) = (s, n);
        while nsec <= -ONE_BILLION && sec > i32::MIN {
            nsec += ONE_BILLION;
            sec -= 1;
        }
        while nsec >= ONE_BILLION && sec < i32::MAX {
            nsec -= ONE_BILLION;
            sec += 1;
        }
        while nsec > 0 && sec < 0 {
            nsec -= ONE_BILLION;
            sec += 1;
        }
        while nsec < 0 && sec > 0 {
            nsec += ONE_BILLION;
            sec -= 1;
        }
        RealTime { sec, nsec }
    }

    /// `RealTime::frame2RealTime`. A zero rate divides by zero in the C++; here it is time zero.
    pub fn frame2real_time(frame: i64, sample_rate: u32) -> RealTime {
        if sample_rate == 0 {
            return RealTime::ZERO;
        }
        if frame < 0 {
            let t = RealTime::frame2real_time(-frame, sample_rate);
            return RealTime::new(-t.sec, -t.nsec);
        }
        let sec = (frame / sample_rate as i64) as i32;
        let frame = frame - sec as i64 * sample_rate as i64;
        let nsec = ((frame as f64 / sample_rate as f64) * ONE_BILLION as f64 + 0.5) as i32;
        // Use ctor here instead of setting data members directly to
        // ensure nsec > ONE_BILLION is handled properly.  It's extremely
        // unlikely, but not impossible.
        RealTime::new(sec, nsec)
    }

    /// `ManagedRealtime::totalMilliseconds`. All integer arithmetic: this truncates to a whole millisecond before
    /// the result becomes a double, and every mark Vixen makes starts from this value.
    pub fn total_milliseconds(&self) -> f64 {
        (self.sec.wrapping_mul(1000).wrapping_add(self.nsec / 1_000_000)) as f64
    }
}

impl Add for RealTime {
    type Output = RealTime;

    fn add(self, r: RealTime) -> RealTime {
        RealTime::new(self.sec.wrapping_add(r.sec), self.nsec.wrapping_add(r.nsec))
    }
}

/// `Vamp::Plugin::Feature`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Feature {
    pub has_timestamp: bool,
    pub timestamp: RealTime,
    pub has_duration: bool,
    pub duration: RealTime,
    pub values: Vec<f32>,
    pub label: String,
}

/// `Vamp::Plugin::FeatureSet`: key is the output number.
pub type FeatureSet = BTreeMap<i32, Vec<Feature>>;

/// `Vamp::Plugin::InputDomain`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InputDomain {
    TimeDomain,
    FrequencyDomain,
}

/// `PluginBase::ParameterDescriptor`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ParameterDescriptor {
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub is_quantized: bool,
    pub quantize_step: f32,
    pub value_names: Vec<String>,
}

/// `OutputDescriptor::SampleType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SampleType {
    #[default]
    OneSamplePerStep,
    FixedSampleRate,
    VariableSampleRate,
}

/// `Plugin::OutputDescriptor`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct OutputDescriptor {
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub has_fixed_bin_count: bool,
    pub bin_count: usize,
    pub bin_names: Vec<String>,
    pub has_known_extents: bool,
    pub min_value: f32,
    pub max_value: f32,
    pub is_quantized: bool,
    pub quantize_step: f32,
    pub sample_type: SampleType,
    pub sample_rate: f32,
    pub has_duration: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_milliseconds_truncates() {
        assert_eq!(RealTime::new(1, 999_999_999).total_milliseconds(), 1999.0);
        assert_eq!(RealTime::new(0, 476_000_000).total_milliseconds(), 476.0);
        assert_eq!(RealTime::frame2real_time(512, 44100).nsec, 11_609_977);
        assert_eq!(RealTime::frame2real_time(512, 44100).total_milliseconds(), 11.0);
    }

    #[test]
    fn frame2real_time_splits_seconds() {
        assert_eq!(RealTime::frame2real_time(44100, 44100), RealTime { sec: 1, nsec: 0 });
        assert_eq!(RealTime::frame2real_time(66150, 44100), RealTime { sec: 1, nsec: 500_000_000 });
        assert_eq!(RealTime::frame2real_time(-22050, 44100), RealTime { sec: 0, nsec: -500_000_000 });
        assert_eq!(RealTime::frame2real_time(10, 0), RealTime::ZERO);
    }

    #[test]
    fn constructor_normalises() {
        assert_eq!(RealTime::new(1, 1_500_000_000), RealTime { sec: 2, nsec: 500_000_000 });
        assert_eq!(RealTime::new(1, -500_000_000), RealTime { sec: 0, nsec: 500_000_000 });
        assert_eq!(RealTime::new(-1, 500_000_000), RealTime { sec: 0, nsec: -500_000_000 });
    }

    #[test]
    fn adding_carries() {
        assert_eq!(
            RealTime::new(1, 600_000_000) + RealTime::new(2, 600_000_000),
            RealTime { sec: 4, nsec: 200_000_000 }
        );
    }
}
