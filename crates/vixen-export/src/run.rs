//! `Export.ProcessExport` and `SequenceIntervalGenerator`: the loop that samples a sequence once per
//! interval and hands each row to a writer.

use crate::{format_audio_file_name, file_name_of, ExportWriter, Result, SessionData};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use vixen_core::TimeSpan;

/// Produces the output values for one point in time. The session layer implements this using the
/// real element schedule and output graph.
pub trait IntervalSource {
    /// Fill `row` with one byte per output for `time`. An output with anything other than an 8-bit
    /// command gets 0, as `ExportCommandHandler` does.
    fn sample(&mut self, time: TimeSpan, row: &mut [u8]);
}

/// What `Export.DoExport` needs to know.
#[derive(Clone, Debug)]
pub struct ExportJob {
    pub sequence_name: String,
    pub sequence_length: TimeSpan,
    pub out_file_name: PathBuf,
    pub update_interval_ms: i32,
    /// One per output in export order: active controllers by index, then each controller's outputs.
    pub channel_names: Vec<String>,
    /// `Export.AudioFilename`: the full path, or empty.
    pub audio_filename: String,
    /// Name the media header after the sequence rather than the original audio file.
    pub match_audio_name: bool,
}

impl ExportJob {
    /// `periods`: whole milliseconds of the sequence, divided by the interval.
    pub fn period_count(&self) -> i32 {
        self.sequence_length.total_ms() as i32 / self.update_interval_ms
    }

    /// The `SequenceSessionData` this job gives the writer.
    pub fn session_data(&self) -> SessionData {
        let output_audio_file_name = if self.audio_filename.is_empty() {
            String::new()
        } else if self.match_audio_name {
            format_audio_file_name(&self.sequence_name, &self.audio_filename)
        } else {
            file_name_of(&self.audio_filename).to_string()
        };
        SessionData {
            out_file_name: self.out_file_name.clone(),
            num_periods: self.period_count(),
            period_ms: self.update_interval_ms,
            channel_names: self.channel_names.clone(),
            time_ms: self.sequence_length.total_ms(),
            audio_file_name: self.audio_filename.clone(),
            output_audio_file_name,
        }
    }
}

/// The result of a finished run. A cancelled run leaves nothing behind, and whatever was at the target before stays as it was.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ExportOutcome {
    pub frames_written: i32,
    pub cancelled: bool,
}

/// Where a run writes until it has finished: `<file>.part`, in the same folder so the rename can't cross a volume.
pub fn part_path(out_file: &Path) -> PathBuf {
    let mut name = out_file.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

/// `ProcessExport`. `progress` gets the percent complete and the task message before an interval, when the whole percent has
/// changed and at most 20 times a second, where the original reports every interval. The first report and 99% are never skipped.
/// Frames are sampled at every multiple of the interval for which a whole interval still fits in the sequence
/// (`SequenceIntervalGenerator.HasNextInterval`).
///
/// FIX: the writer goes to [`part_path`], which is renamed over the target only on success. On a cancel or an error it's deleted,
/// so an earlier export stays untouched. Upstream truncates the target when the run starts and leaves whatever it got to, with a
/// header that declares more blocks than its table holds.
pub fn run_export(
    job: &ExportJob,
    writer: &mut dyn ExportWriter,
    source: &mut dyn IntervalSource,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(i32, &str),
) -> Result<ExportOutcome> {
    let mut session = job.session_data();
    let target = session.out_file_name.clone();
    let part = part_path(&target);
    session.out_file_name = part.clone();
    let periods = session.num_periods;
    let interval = TimeSpan::from_ms_i(job.update_interval_ms as i64);
    let length = job.sequence_length;
    let message = format!("Exporting {}", job.sequence_name);

    let mut row = vec![0u8; job.channel_names.len()];
    let mut position = TimeSpan::ZERO;
    let mut written = 0;

    if cancel.load(Ordering::Relaxed) {
        // The original checks this before it opens the session, and leaves no file behind.
        return Ok(ExportOutcome { frames_written: 0, cancelled: true });
    }

    let result = (|| -> Result<ExportOutcome> {
        writer.open_session(&session)?;
        let mut reported = -1;
        let mut last_report = Instant::now();
        while position + interval <= length && !cancel.load(Ordering::Relaxed) {
            // A zero period count divides by zero in floating point: not a number casts to int's minimum
            // in .NET, but this branch can't be reached then, since no interval fits.
            let percent = (written as f64 / periods as f64 * 100.0) as i32;
            if percent != reported && (reported < 0 || percent >= 99 || last_report.elapsed() >= PROGRESS_INTERVAL) {
                progress(percent, &message);
                reported = percent;
                last_report = Instant::now();
            }
            source.sample(position, &mut row);
            writer.write_next_period(&row)?;
            position = position + interval;
            written += 1;
        }
        let cancelled = cancel.load(Ordering::Relaxed);
        if !cancelled {
            writer.close_session()?;
            std::fs::rename(&part, &target)?;
        }
        Ok(ExportOutcome { frames_written: written, cancelled })
    })();
    if !matches!(result, Ok(ExportOutcome { cancelled: false, .. })) {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fseq::FseqV2Writer;
    use crate::{ExportError, ExportFormat};
    use std::fs;

    struct Counting(u8);
    impl IntervalSource for Counting {
        fn sample(&mut self, _: TimeSpan, row: &mut [u8]) {
            row.fill(self.0);
            self.0 = self.0.wrapping_add(1);
        }
    }

    /// Cancels when the source reaches a frame.
    struct CancelAt<'a> {
        frame: i32,
        seen: i32,
        cancel: &'a AtomicBool,
    }
    impl IntervalSource for CancelAt<'_> {
        fn sample(&mut self, _: TimeSpan, row: &mut [u8]) {
            row.fill(1);
            self.seen += 1;
            if self.seen == self.frame {
                self.cancel.store(true, Ordering::Relaxed);
            }
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-export-run-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn job(out: &Path, length_ms: f64, interval: i32) -> ExportJob {
        ExportJob {
            sequence_name: "Song".to_string(),
            sequence_length: TimeSpan::from_ms(length_ms),
            out_file_name: out.to_path_buf(),
            update_interval_ms: interval,
            channel_names: vec![String::new(); 4],
            audio_filename: String::new(),
            match_audio_name: false,
        }
    }

    #[test]
    fn a_finished_run_leaves_the_target_and_no_part_file() {
        let dir = temp("finish");
        let out = dir.join("Song.fseq");
        let cancel = AtomicBool::new(false);
        let mut writer = FseqV2Writer::new(true);
        let outcome = run_export(&job(&out, 1000.0, 25), &mut writer, &mut Counting(0), &cancel, &mut |_, _| {}).unwrap();
        assert_eq!(outcome, ExportOutcome { frames_written: 40, cancelled: false });
        assert!(out.is_file());
        assert!(!part_path(&out).exists());
    }

    #[test]
    fn a_cancel_leaves_an_earlier_export_exactly_as_it_was() {
        let dir = temp("cancel");
        let out = dir.join("Song.fseq");
        fs::write(&out, b"the earlier export").unwrap();
        let cancel = AtomicBool::new(false);
        let mut source = CancelAt { frame: 3, seen: 0, cancel: &cancel };
        let mut writer = FseqV2Writer::new(true);
        let outcome = run_export(&job(&out, 1000.0, 25), &mut writer, &mut source, &cancel, &mut |_, _| {}).unwrap();
        assert!(outcome.cancelled);
        assert_eq!(outcome.frames_written, 3);
        assert_eq!(fs::read(&out).unwrap(), b"the earlier export");
        assert!(!part_path(&out).exists());
    }

    #[test]
    fn a_cancel_before_any_frame_writes_no_file() {
        let dir = temp("cancel-first");
        let out = dir.join("Song.fseq");
        let cancel = AtomicBool::new(true);
        let mut writer = FseqV2Writer::new(true);
        let outcome = run_export(&job(&out, 1000.0, 25), &mut writer, &mut Counting(0), &cancel, &mut |_, _| {}).unwrap();
        assert!(outcome.cancelled);
        assert!(!out.exists() && !part_path(&out).exists());
    }

    #[test]
    fn a_failing_writer_is_an_error_and_cleans_up() {
        struct Failing;
        impl ExportWriter for Failing {
            fn open_session(&mut self, data: &SessionData) -> Result<()> {
                fs::write(&data.out_file_name, b"partial")?;
                Ok(())
            }
            fn write_next_period(&mut self, _: &[u8]) -> Result<()> {
                Err(ExportError::Format("disk full".to_string()))
            }
            fn close_session(&mut self) -> Result<()> {
                Ok(())
            }
        }
        let dir = temp("fail");
        let out = dir.join("Song.fseq");
        fs::write(&out, b"the earlier export").unwrap();
        let cancel = AtomicBool::new(false);
        let error = run_export(&job(&out, 1000.0, 25), &mut Failing, &mut Counting(0), &cancel, &mut |_, _| {}).unwrap_err();
        assert_eq!(error.to_string(), "disk full");
        assert_eq!(fs::read(&out).unwrap(), b"the earlier export");
        assert!(!part_path(&out).exists());
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_an_error_not_a_panic() {
        let dir = temp("no-folder");
        let out = dir.join("missing").join("Song.fseq");
        let cancel = AtomicBool::new(false);
        for format in ExportFormat::ALL {
            let mut writer = format.writer(true);
            assert!(run_export(&job(&out, 1000.0, 25), writer.as_mut(), &mut Counting(0), &cancel, &mut |_, _| {}).is_err(), "{format:?}");
        }
    }

    #[test]
    fn progress_reports_rise_and_reach_99_with_no_repeats() {
        let dir = temp("progress");
        let out = dir.join("Song.csv");
        let cancel = AtomicBool::new(false);
        let mut writer = ExportFormat::Csv.writer(false);
        let mut seen = Vec::new();
        run_export(&job(&out, 5000.0, 25), writer.as_mut(), &mut Counting(0), &cancel, &mut |percent, message| {
            assert_eq!(message, "Exporting Song");
            seen.push(percent);
        })
        .unwrap();
        assert_eq!(seen.first(), Some(&0));
        assert_eq!(seen.last(), Some(&99));
        assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
    }

    #[test]
    fn frames_follow_the_length_and_interval() {
        let dir = temp("frames");
        let cancel = AtomicBool::new(false);
        let count = |length: f64, interval: i32| {
            let out = dir.join("x.csv");
            let mut writer = ExportFormat::Csv.writer(false);
            run_export(&job(&out, length, interval), writer.as_mut(), &mut Counting(0), &cancel, &mut |_, _| {}).unwrap().frames_written
        };
        assert_eq!(count(148_665.0, 25), 5946);
        assert_eq!(count(100.5, 25), 4);
        assert_eq!(count(24.0, 25), 0);
    }
}
