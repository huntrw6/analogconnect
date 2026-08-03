use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Mutex,
    time::Instant,
};

use analogconnect_core::{AudioFormat, AudioFrame};
use serde::Serialize;
use thiserror::Error;

struct QueuedFrame {
    frame: AudioFrame,
    enqueued_at: Instant,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AudioQueueSummary {
    pub depth: usize,
    pub enqueued: u64,
    pub dequeued: u64,
    pub dropped: u64,
    pub max_observed_latency_micros: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AudioBridgeSummary {
    pub uplink: AudioQueueSummary,
    pub downlink: AudioQueueSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct JitterBufferSummary {
    pub depth: usize,
    pub received: u64,
    pub emitted: u64,
    pub duplicate: u64,
    pub late: u64,
    pub missing: u64,
    pub overflow: u64,
}

pub struct JitterBuffer {
    capacity: usize,
    target_depth: usize,
    frames: BTreeMap<u64, AudioFrame>,
    next_sequence: Option<u64>,
    started: bool,
    summary: JitterBufferSummary,
}

impl JitterBuffer {
    pub fn new(capacity: usize, target_depth: usize) -> Result<Self, JitterBufferError> {
        if capacity == 0 || target_depth == 0 || target_depth > capacity {
            return Err(JitterBufferError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            target_depth,
            frames: BTreeMap::new(),
            next_sequence: None,
            started: false,
            summary: JitterBufferSummary::default(),
        })
    }

    pub fn insert(&mut self, frame: AudioFrame) {
        self.summary.received = self.summary.received.saturating_add(1);
        let sequence = frame.sequence();
        if self
            .next_sequence
            .is_some_and(|next_sequence| sequence < next_sequence)
        {
            self.summary.late = self.summary.late.saturating_add(1);
            return;
        }
        if self.frames.contains_key(&sequence) {
            self.summary.duplicate = self.summary.duplicate.saturating_add(1);
            return;
        }
        if self.frames.len() == self.capacity {
            self.summary.overflow = self.summary.overflow.saturating_add(1);
            let furthest = *self.frames.last_key_value().unwrap().0;
            if sequence >= furthest {
                return;
            }
            self.frames.remove(&furthest);
        }
        self.frames.insert(sequence, frame);
        self.summary.depth = self.frames.len();
    }

    pub fn pop(&mut self) -> Option<AudioFrame> {
        if !self.started {
            if self.frames.len() < self.target_depth {
                return None;
            }
            self.next_sequence = self.frames.first_key_value().map(|(sequence, _)| *sequence);
            self.started = true;
        }
        let sequence = self.next_sequence?;
        self.next_sequence = sequence.checked_add(1);
        if let Some(frame) = self.frames.remove(&sequence) {
            self.summary.emitted = self.summary.emitted.saturating_add(1);
            self.summary.depth = self.frames.len();
            Some(frame)
        } else {
            if !self.frames.is_empty() {
                self.summary.missing = self.summary.missing.saturating_add(1);
            }
            None
        }
    }

    #[must_use]
    pub fn summary(&self) -> JitterBufferSummary {
        self.summary.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum JitterBufferError {
    #[error("jitter buffer capacity and target depth must be valid")]
    InvalidCapacity,
}

pub trait PipeWireDumpRunner: Send + Sync {
    type Error;

    /// Returns a transient PipeWire snapshot. Implementations and callers must
    /// not log or persist it because unrelated properties can contain addresses.
    fn dump(&self) -> Result<String, Self::Error>;
}

pub struct PwDumpRunner {
    executable: PathBuf,
}

impl PwDumpRunner {
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for PwDumpRunner {
    fn default() -> Self {
        Self::new("pw-dump")
    }
}

impl PipeWireDumpRunner for PwDumpRunner {
    type Error = ();

    fn dump(&self) -> Result<String, Self::Error> {
        let output = Command::new(&self.executable)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|_| ())?;
        if !output.status.success() {
            return Err(());
        }
        String::from_utf8(output.stdout).map_err(|_| ())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ScoNodePair {
    /// Captures iPhone audio from PipeWire for Android downlink.
    pub source_serial: u32,
    /// Plays Android uplink into PipeWire toward the iPhone.
    pub sink_serial: u32,
}

impl std::fmt::Debug for ScoNodePair {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScoNodePair")
            .field("source_serial", &self.source_serial)
            .field("sink_serial", &self.sink_serial)
            .finish()
    }
}

pub struct ScoNodeLocator<R> {
    runner: R,
}

impl<R> ScoNodeLocator<R>
where
    R: PipeWireDumpRunner,
{
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    pub fn locate(&self) -> Result<ScoNodePair, ScoNodeError> {
        let dump = self.runner.dump().map_err(|_| ScoNodeError::Unavailable)?;
        let objects: serde_json::Value =
            serde_json::from_str(&dump).map_err(|_| ScoNodeError::InvalidSnapshot)?;
        let objects = objects.as_array().ok_or(ScoNodeError::InvalidSnapshot)?;
        let mut sources = Vec::new();
        let mut sinks = Vec::new();
        for object in objects {
            if object.get("type").and_then(|value| value.as_str())
                != Some("PipeWire:Interface:Node")
            {
                continue;
            }
            let Some(serial) = object
                .pointer("/info/props/object.serial")
                .and_then(parse_pipewire_serial)
            else {
                continue;
            };
            match object
                .pointer("/info/props/factory.name")
                .and_then(|value| value.as_str())
            {
                Some("api.bluez5.sco.source") => sources.push(serial),
                Some("api.bluez5.sco.sink") => sinks.push(serial),
                _ => {}
            }
        }
        sources.sort_unstable();
        sources.dedup();
        sinks.sort_unstable();
        sinks.dedup();
        if sources.len() != 1 || sinks.len() != 1 || sources[0] == sinks[0] {
            return Err(ScoNodeError::Ambiguous);
        }
        Ok(ScoNodePair {
            source_serial: sources[0],
            sink_serial: sinks[0],
        })
    }
}

fn parse_pipewire_serial(value: &serde_json::Value) -> Option<u32> {
    value
        .as_str()
        .and_then(|value| value.parse().ok())
        .or_else(|| value.as_u64().and_then(|value| value.try_into().ok()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ScoNodeError {
    #[error("PipeWire snapshot is unavailable")]
    Unavailable,
    #[error("PipeWire snapshot is invalid")]
    InvalidSnapshot,
    #[error("PipeWire SCO node state is absent or ambiguous")]
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PwCatDirection {
    Capture,
    Playback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PwCatCommand {
    direction: PwCatDirection,
    target_serial: u32,
    sample_rate_hz: u32,
}

impl PwCatCommand {
    fn new(
        direction: PwCatDirection,
        target_serial: u32,
        format: AudioFormat,
    ) -> Result<Self, PwCatStreamError> {
        if !matches!(
            format,
            AudioFormat::HFP_NARROWBAND | AudioFormat::HFP_WIDEBAND
        ) {
            return Err(PwCatStreamError::UnsupportedFormat);
        }
        Ok(Self {
            direction,
            target_serial,
            sample_rate_hz: format.sample_rate_hz,
        })
    }

    fn arguments(&self) -> Vec<String> {
        vec![
            match self.direction {
                PwCatDirection::Capture => "--record",
                PwCatDirection::Playback => "--playback",
            }
            .to_owned(),
            "--raw".to_owned(),
            "--target".to_owned(),
            self.target_serial.to_string(),
            "--rate".to_owned(),
            self.sample_rate_hz.to_string(),
            "--channels".to_owned(),
            "1".to_owned(),
            "--format".to_owned(),
            "s16".to_owned(),
            "--latency".to_owned(),
            "7.5ms".to_owned(),
            "-".to_owned(),
        ]
    }
}

/// Owns the two live PipeWire processes for one call. PCM stays in anonymous
/// pipes: callers must not persist or log bytes read from or written to them.
pub struct PwCatSession {
    capture: Child,
    playback: Child,
}

impl PwCatSession {
    pub fn start(
        executable: impl Into<PathBuf>,
        nodes: ScoNodePair,
        format: AudioFormat,
    ) -> Result<Self, PwCatStreamError> {
        let executable = executable.into();
        let capture_spec = PwCatCommand::new(PwCatDirection::Capture, nodes.source_serial, format)?;
        let playback_spec = PwCatCommand::new(PwCatDirection::Playback, nodes.sink_serial, format)?;

        let mut capture = spawn_pw_cat(&executable, &capture_spec)
            .map_err(|_| PwCatStreamError::CaptureStartFailed)?;
        let playback = match spawn_pw_cat(&executable, &playback_spec) {
            Ok(child) => child,
            Err(_) => {
                stop_child(&mut capture);
                return Err(PwCatStreamError::PlaybackStartFailed);
            }
        };
        if capture.stdout.is_none() || playback.stdin.is_none() {
            let mut playback = playback;
            stop_child(&mut capture);
            stop_child(&mut playback);
            return Err(PwCatStreamError::PipeUnavailable);
        }
        Ok(Self { capture, playback })
    }

    pub fn capture_reader(&mut self) -> Result<&mut ChildStdout, PwCatStreamError> {
        self.capture
            .stdout
            .as_mut()
            .ok_or(PwCatStreamError::PipeUnavailable)
    }

    pub fn playback_writer(&mut self) -> Result<&mut ChildStdin, PwCatStreamError> {
        self.playback
            .stdin
            .as_mut()
            .ok_or(PwCatStreamError::PipeUnavailable)
    }
}

impl Drop for PwCatSession {
    fn drop(&mut self) {
        stop_child(&mut self.capture);
        stop_child(&mut self.playback);
    }
}

fn spawn_pw_cat(executable: &PathBuf, spec: &PwCatCommand) -> std::io::Result<Child> {
    let mut command = Command::new(executable);
    command.args(spec.arguments()).stderr(Stdio::null());
    match spec.direction {
        PwCatDirection::Capture => command.stdin(Stdio::null()).stdout(Stdio::piped()),
        PwCatDirection::Playback => command.stdin(Stdio::piped()).stdout(Stdio::null()),
    };
    command.spawn()
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PwCatStreamError {
    #[error("audio format is unsupported")]
    UnsupportedFormat,
    #[error("PipeWire capture process could not start")]
    CaptureStartFailed,
    #[error("PipeWire playback process could not start")]
    PlaybackStartFailed,
    #[error("PipeWire audio pipe is unavailable")]
    PipeUnavailable,
}

struct QueueState {
    frames: VecDeque<QueuedFrame>,
    summary: AudioQueueSummary,
}

pub struct BoundedAudioQueue {
    capacity: usize,
    state: Mutex<QueueState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AudioQueueError {
    #[error("audio queue capacity must be greater than zero")]
    ZeroCapacity,
    #[error("audio queue lock was poisoned")]
    LockPoisoned,
}

impl BoundedAudioQueue {
    pub fn new(capacity: usize) -> Result<Self, AudioQueueError> {
        if capacity == 0 {
            return Err(AudioQueueError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            state: Mutex::new(QueueState {
                frames: VecDeque::with_capacity(capacity),
                summary: AudioQueueSummary::default(),
            }),
        })
    }

    pub fn push(&self, frame: AudioFrame) -> Result<(), AudioQueueError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AudioQueueError::LockPoisoned)?;
        if state.frames.len() == self.capacity {
            state.frames.pop_front();
            state.summary.dropped = state.summary.dropped.saturating_add(1);
        }
        state.frames.push_back(QueuedFrame {
            frame,
            enqueued_at: Instant::now(),
        });
        state.summary.enqueued = state.summary.enqueued.saturating_add(1);
        state.summary.depth = state.frames.len();
        Ok(())
    }

    pub fn pop(&self) -> Result<Option<AudioFrame>, AudioQueueError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AudioQueueError::LockPoisoned)?;
        let queued = state.frames.pop_front();
        if let Some(queued) = queued {
            let latency =
                u64::try_from(queued.enqueued_at.elapsed().as_micros()).unwrap_or(u64::MAX);
            state.summary.dequeued = state.summary.dequeued.saturating_add(1);
            state.summary.max_observed_latency_micros =
                state.summary.max_observed_latency_micros.max(latency);
            state.summary.depth = state.frames.len();
            Ok(Some(queued.frame))
        } else {
            Ok(None)
        }
    }

    pub fn summary(&self) -> Result<AudioQueueSummary, AudioQueueError> {
        self.state
            .lock()
            .map(|state| state.summary.clone())
            .map_err(|_| AudioQueueError::LockPoisoned)
    }
}

pub struct AudioBridge {
    pub uplink: BoundedAudioQueue,
    pub downlink: BoundedAudioQueue,
}

impl AudioBridge {
    pub fn new(frames_per_direction: usize) -> Result<Self, AudioQueueError> {
        Ok(Self {
            uplink: BoundedAudioQueue::new(frames_per_direction)?,
            downlink: BoundedAudioQueue::new(frames_per_direction)?,
        })
    }

    pub fn summary(&self) -> Result<AudioBridgeSummary, AudioQueueError> {
        Ok(AudioBridgeSummary {
            uplink: self.uplink.summary()?,
            downlink: self.downlink.summary()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(sequence: u64) -> AudioFrame {
        AudioFrame::new(
            sequence,
            AudioFormat::HFP_WIDEBAND,
            vec![0; usize::from(AudioFormat::HFP_WIDEBAND.samples_per_channel)],
        )
        .unwrap()
    }

    #[test]
    fn overflow_drops_oldest_frame_to_bound_latency() {
        let queue = BoundedAudioQueue::new(2).unwrap();
        queue.push(frame(1)).unwrap();
        queue.push(frame(2)).unwrap();
        queue.push(frame(3)).unwrap();
        assert_eq!(queue.pop().unwrap().unwrap().sequence(), 2);
        let summary = queue.summary().unwrap();
        assert_eq!(summary.dropped, 1);
        assert_eq!(summary.depth, 1);
    }

    #[test]
    fn bridge_tracks_directions_independently() {
        let bridge = AudioBridge::new(4).unwrap();
        bridge.uplink.push(frame(1)).unwrap();
        bridge.downlink.push(frame(2)).unwrap();
        bridge.downlink.pop().unwrap();
        let summary = bridge.summary().unwrap();
        assert_eq!(summary.uplink.depth, 1);
        assert_eq!(summary.downlink.depth, 0);
        assert_eq!(summary.downlink.dequeued, 1);
    }

    #[test]
    fn jitter_buffer_reorders_before_playout() {
        let mut buffer = JitterBuffer::new(4, 3).unwrap();
        buffer.insert(frame(2));
        buffer.insert(frame(1));
        assert!(buffer.pop().is_none());
        buffer.insert(frame(3));
        assert_eq!(buffer.pop().unwrap().sequence(), 1);
        assert_eq!(buffer.pop().unwrap().sequence(), 2);
        assert_eq!(buffer.pop().unwrap().sequence(), 3);
        assert_eq!(buffer.summary().emitted, 3);
    }

    #[test]
    fn jitter_buffer_counts_loss_duplicates_and_late_frames() {
        let mut buffer = JitterBuffer::new(4, 2).unwrap();
        buffer.insert(frame(10));
        buffer.insert(frame(12));
        buffer.insert(frame(12));
        assert_eq!(buffer.pop().unwrap().sequence(), 10);
        assert!(buffer.pop().is_none());
        buffer.insert(frame(9));
        assert_eq!(buffer.pop().unwrap().sequence(), 12);
        let summary = buffer.summary();
        assert_eq!(summary.missing, 1);
        assert_eq!(summary.duplicate, 1);
        assert_eq!(summary.late, 1);
    }

    #[test]
    fn jitter_buffer_bounds_future_latency() {
        let mut buffer = JitterBuffer::new(2, 1).unwrap();
        buffer.insert(frame(5));
        buffer.insert(frame(7));
        buffer.insert(frame(6));
        assert_eq!(buffer.summary().overflow, 1);
        assert_eq!(buffer.pop().unwrap().sequence(), 5);
        assert_eq!(buffer.pop().unwrap().sequence(), 6);
    }

    struct FixtureDumpRunner {
        dump: String,
    }

    impl PipeWireDumpRunner for FixtureDumpRunner {
        type Error = ();

        fn dump(&self) -> Result<String, Self::Error> {
            Ok(self.dump.clone())
        }
    }

    #[test]
    fn sco_locator_uses_only_factory_and_numeric_serials() {
        let private_marker = "PRIVATE-BLUETOOTH-MARKER";
        let dump = format!(
            r#"[
              {{"id":71,"type":"PipeWire:Interface:Node","info":{{"props":{{
                "object.serial":"701",
                "factory.name":"api.bluez5.sco.source",
                "node.name":"{private_marker}"}}}}}},
              {{"id":72,"type":"PipeWire:Interface:Node","info":{{"props":{{
                "object.serial":"702",
                "factory.name":"api.bluez5.sco.sink",
                "api.bluez5.address":"{private_marker}"}}}}}},
              {{"id":73,"type":"PipeWire:Interface:Node","info":{{"props":{{
                "factory.name":"api.alsa.pcm.sink"}}}}}}
            ]"#
        );
        let nodes = ScoNodeLocator::new(FixtureDumpRunner { dump })
            .locate()
            .unwrap();
        assert_eq!(nodes.source_serial, 701);
        assert_eq!(nodes.sink_serial, 702);
        assert!(!format!("{nodes:?}").contains(private_marker));
    }

    #[test]
    fn sco_locator_fails_closed_on_missing_duplicate_or_invalid_nodes() {
        for dump in [
            "[]",
            r#"[
              {"id":1,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"11","factory.name":"api.bluez5.sco.source"}}},
              {"id":2,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"12","factory.name":"api.bluez5.sco.source"}}},
              {"id":3,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"13","factory.name":"api.bluez5.sco.sink"}}}
            ]"#,
        ] {
            assert_eq!(
                ScoNodeLocator::new(FixtureDumpRunner {
                    dump: dump.to_owned()
                })
                .locate(),
                Err(ScoNodeError::Ambiguous)
            );
        }
        assert_eq!(
            ScoNodeLocator::new(FixtureDumpRunner {
                dump: "private malformed payload".to_owned()
            })
            .locate(),
            Err(ScoNodeError::InvalidSnapshot)
        );
    }

    #[test]
    fn pw_cat_commands_bind_correct_directions_and_pcm_formats() {
        let nodes = ScoNodePair {
            source_serial: 701,
            sink_serial: 702,
        };
        for (format, rate) in [
            (AudioFormat::HFP_NARROWBAND, "8000"),
            (AudioFormat::HFP_WIDEBAND, "16000"),
        ] {
            let capture =
                PwCatCommand::new(PwCatDirection::Capture, nodes.source_serial, format).unwrap();
            let playback =
                PwCatCommand::new(PwCatDirection::Playback, nodes.sink_serial, format).unwrap();
            assert_eq!(
                capture.arguments(),
                [
                    "--record",
                    "--raw",
                    "--target",
                    "701",
                    "--rate",
                    rate,
                    "--channels",
                    "1",
                    "--format",
                    "s16",
                    "--latency",
                    "7.5ms",
                    "-",
                ]
            );
            assert_eq!(playback.arguments()[0], "--playback");
            assert_eq!(playback.arguments()[3], "702");
        }
    }

    #[test]
    fn pw_cat_rejects_non_hfp_format_without_disclosing_paths() {
        let unsupported = AudioFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            samples_per_channel: 360,
        };
        let error = PwCatCommand::new(PwCatDirection::Capture, 7, unsupported).unwrap_err();
        assert_eq!(error, PwCatStreamError::UnsupportedFormat);
        assert!(!format!("{error:?}").contains('/'));
    }
}
