use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use analogconnect_core::{AudioFormat, AudioFrame, AudioPacket, AudioTransportState, CallState};
use serde::Serialize;
use thiserror::Error;

use crate::process::run_bounded;

const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const MAX_HELPER_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

pub struct ScoTeardownWatchdog {
    grace_period: std::time::Duration,
    inconsistent_since: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoTeardownObservation {
    pub state: AudioTransportState,
    pub stalled: bool,
}

impl ScoTeardownWatchdog {
    #[must_use]
    pub const fn new(grace_period: std::time::Duration) -> Self {
        Self {
            grace_period,
            inconsistent_since: None,
        }
    }

    pub fn observe(
        &mut self,
        call: CallState,
        audio: AudioTransportState,
        now: Instant,
    ) -> ScoTeardownObservation {
        if matches!(call, CallState::Idle | CallState::Ended)
            && audio == AudioTransportState::ScoActive
        {
            let since = *self.inconsistent_since.get_or_insert(now);
            let stalled = now.saturating_duration_since(since) >= self.grace_period;
            return ScoTeardownObservation {
                state: if stalled {
                    AudioTransportState::Error
                } else {
                    AudioTransportState::ScoTearingDown
                },
                stalled,
            };
        }
        self.inconsistent_since = None;
        ScoTeardownObservation {
            state: audio,
            stalled: false,
        }
    }
}

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

    /// Advances one playout interval after the target depth has been reached.
    /// Call exactly once per negotiated HFP frame duration (currently 7.5 ms).
    pub fn tick(&mut self) -> Option<AudioFrame> {
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
            self.summary.missing = self.summary.missing.saturating_add(1);
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
        let output = run_bounded(
            &self.executable,
            std::iter::empty::<&str>(),
            HELPER_TIMEOUT,
            MAX_HELPER_OUTPUT_BYTES,
        )
        .map_err(|_| ())?;
        String::from_utf8(output).map_err(|_| ())
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
        if sources.is_empty() && sinks.is_empty() {
            return Err(ScoNodeError::Absent);
        }
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
    #[error("PipeWire SCO nodes are absent")]
    Absent,
    #[error("PipeWire SCO node state is absent or ambiguous")]
    Ambiguous,
}

pub trait AudioStateBackend: Send + Sync {
    type Error;

    fn snapshot(&self) -> Result<AudioTransportState, Self::Error>;
}

pub struct PipeWireAudioStateBackend<R> {
    locator: ScoNodeLocator<R>,
}

impl<R: PipeWireDumpRunner> PipeWireAudioStateBackend<R> {
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self {
            locator: ScoNodeLocator::new(runner),
        }
    }
}

impl Default for PipeWireAudioStateBackend<PwDumpRunner> {
    fn default() -> Self {
        Self::new(PwDumpRunner::default())
    }
}

impl<R> AudioStateBackend for PipeWireAudioStateBackend<R>
where
    R: PipeWireDumpRunner,
{
    type Error = ScoNodeError;

    fn snapshot(&self) -> Result<AudioTransportState, Self::Error> {
        match self.locator.locate() {
            Ok(_) => Ok(AudioTransportState::ScoActive),
            Err(ScoNodeError::Absent) => Ok(AudioTransportState::Inactive),
            Err(error) => Err(error),
        }
    }
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
    format: AudioFormat,
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
        Ok(Self {
            capture,
            playback,
            format,
        })
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

    /// Transfers both anonymous PCM pipes into independently movable framing
    /// adapters. The session must remain alive to own and reap both processes.
    pub fn take_frame_streams(&mut self) -> Result<PwCatFrameStreams, PwCatStreamError> {
        if self.capture.stdout.is_none() || self.playback.stdin.is_none() {
            return Err(PwCatStreamError::PipeUnavailable);
        }
        let capture = self.capture.stdout.take().unwrap();
        let playback = self.playback.stdin.take().unwrap();
        Ok(PwCatFrameStreams {
            downlink: PcmFrameReader::new(capture, self.format)
                .map_err(|_| PwCatStreamError::PipeUnavailable)?,
            uplink: PcmFrameWriter::new(playback, self.format)
                .map_err(|_| PwCatStreamError::PipeUnavailable)?,
        })
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

/// Framed call-audio directions that can be moved to separate worker threads.
pub struct PwCatFrameStreams {
    pub downlink: PcmFrameReader<ChildStdout>,
    pub uplink: PcmFrameWriter<ChildStdin>,
}

impl PwCatFrameStreams {
    #[must_use]
    pub fn into_parts(self) -> (PcmFrameReader<ChildStdout>, PcmFrameWriter<ChildStdin>) {
        (self.downlink, self.uplink)
    }
}

const LIVE_BRIDGE_OK: u8 = 0;
const LIVE_BRIDGE_CAPTURE_FAILED: u8 = 1;
const LIVE_BRIDGE_PLAYBACK_FAILED: u8 = 2;

/// Owns `pw-cat`, its anonymous PCM pipes, and one worker per audio direction.
/// Dropping it kills the child processes before joining workers so blocked pipe
/// operations are released without persisting or logging any samples.
pub struct LiveAudioBridge {
    session: Option<PwCatSession>,
    stop: Arc<AtomicBool>,
    failure: Arc<AtomicU8>,
    workers: Vec<JoinHandle<()>>,
}

impl LiveAudioBridge {
    pub fn start(
        executable: impl Into<PathBuf>,
        nodes: ScoNodePair,
        format: AudioFormat,
        bridge: Arc<AudioBridge>,
    ) -> Result<Self, LiveAudioBridgeError> {
        let mut session =
            PwCatSession::start(executable, nodes, format).map_err(LiveAudioBridgeError::Stream)?;
        let streams = session
            .take_frame_streams()
            .map_err(LiveAudioBridgeError::Stream)?;
        let (mut capture, mut playback) = streams.into_parts();
        let stop = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(AtomicU8::new(LIVE_BRIDGE_OK));

        let capture_stop = Arc::clone(&stop);
        let capture_failure = Arc::clone(&failure);
        let capture_bridge = Arc::clone(&bridge);
        let capture_worker = thread::Builder::new()
            .name("analogconnect-sco-capture".to_owned())
            .spawn(move || {
                while !capture_stop.load(Ordering::Acquire) {
                    match capture_into_bridge(&mut capture, &capture_bridge) {
                        Ok(true) => {}
                        Ok(false) => break,
                        _ => {
                            capture_failure.store(LIVE_BRIDGE_CAPTURE_FAILED, Ordering::Release);
                            break;
                        }
                    }
                }
            })
            .map_err(|_| LiveAudioBridgeError::WorkerStartFailed)?;

        let playback_stop = Arc::clone(&stop);
        let playback_failure = Arc::clone(&failure);
        let playback_worker = match thread::Builder::new()
            .name("analogconnect-sco-playback".to_owned())
            .spawn(move || {
                let silence =
                    AudioFrame::new(0, format, vec![0; usize::from(format.samples_per_channel)])
                        .expect("fixed HFP silence frame is valid");
                let frame_period = Duration::from_micros(format.frame_duration_micros());
                let mut deadline = Instant::now();
                while !playback_stop.load(Ordering::Acquire) {
                    deadline += frame_period;
                    if playback_from_bridge(&mut playback, &bridge, &silence).is_err() {
                        playback_failure.store(LIVE_BRIDGE_PLAYBACK_FAILED, Ordering::Release);
                        break;
                    }
                    if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                        thread::sleep(remaining);
                    } else {
                        deadline = Instant::now();
                    }
                }
            }) {
            Ok(worker) => worker,
            Err(_) => {
                stop.store(true, Ordering::Release);
                drop(session);
                let _ = capture_worker.join();
                return Err(LiveAudioBridgeError::WorkerStartFailed);
            }
        };

        Ok(Self {
            session: Some(session),
            stop,
            failure,
            workers: vec![capture_worker, playback_worker],
        })
    }

    #[must_use]
    pub fn failure_code(&self) -> Option<&'static str> {
        match self.failure.load(Ordering::Acquire) {
            LIVE_BRIDGE_CAPTURE_FAILED => Some("sco_capture_failed"),
            LIVE_BRIDGE_PLAYBACK_FAILED => Some("sco_playback_failed"),
            _ => None,
        }
    }
}

fn capture_into_bridge<R: Read>(
    capture: &mut PcmFrameReader<R>,
    bridge: &AudioBridge,
) -> Result<bool, LiveAudioBridgeError> {
    match capture.read_frame().map_err(LiveAudioBridgeError::Pcm)? {
        Some(frame) => {
            bridge
                .downlink
                .push(frame)
                .map_err(|_| LiveAudioBridgeError::QueueUnavailable)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

fn playback_from_bridge<W: Write>(
    playback: &mut PcmFrameWriter<W>,
    bridge: &AudioBridge,
    silence: &AudioFrame,
) -> Result<(), LiveAudioBridgeError> {
    let frame = bridge
        .uplink
        .pop()
        .map_err(|_| LiveAudioBridgeError::QueueUnavailable)?
        .unwrap_or_else(|| silence.clone());
    playback
        .write_frame(&frame)
        .map_err(LiveAudioBridgeError::Pcm)
}

impl Drop for LiveAudioBridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        drop(self.session.take());
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl std::fmt::Debug for LiveAudioBridge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveAudioBridge")
            .field("failure_code", &self.failure_code())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
pub enum LiveAudioBridgeError {
    #[error(transparent)]
    Stream(#[from] PwCatStreamError),
    #[error("call-audio worker could not start")]
    WorkerStartFailed,
    #[error(transparent)]
    Pcm(#[from] PcmStreamError),
    #[error("call-audio queue is unavailable")]
    QueueUnavailable,
}

/// Converts the raw little-endian PCM emitted by `pw-cat` into exact HFP frames.
/// Audio bytes are never retained after the returned frame is constructed.
pub struct PcmFrameReader<R> {
    reader: R,
    format: AudioFormat,
    next_sequence: u64,
}

impl<R: Read> PcmFrameReader<R> {
    pub fn new(reader: R, format: AudioFormat) -> Result<Self, PcmStreamError> {
        validate_hfp_format(format)?;
        Ok(Self {
            reader,
            format,
            next_sequence: 0,
        })
    }

    pub fn read_frame(&mut self) -> Result<Option<AudioFrame>, PcmStreamError> {
        let sample_count = usize::from(self.format.samples_per_channel);
        let mut bytes = vec![0_u8; sample_count * size_of::<i16>()];
        let mut filled = 0;
        while filled < bytes.len() {
            match self.reader.read(&mut bytes[filled..]) {
                Ok(0) if filled == 0 => return Ok(None),
                Ok(0) => return Err(PcmStreamError::TruncatedFrame),
                Ok(count) => filled += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return Err(PcmStreamError::ReadFailed),
            }
        }
        let samples = bytes
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(PcmStreamError::SequenceExhausted)?;
        AudioFrame::new(sequence, self.format, samples)
            .map(Some)
            .map_err(|_| PcmStreamError::InvalidFrame)
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}

/// Writes exact HFP frames to the raw little-endian PCM accepted by `pw-cat`.
pub struct PcmFrameWriter<W> {
    writer: W,
    format: AudioFormat,
}

impl<W: Write> PcmFrameWriter<W> {
    pub fn new(writer: W, format: AudioFormat) -> Result<Self, PcmStreamError> {
        validate_hfp_format(format)?;
        Ok(Self { writer, format })
    }

    pub fn write_frame(&mut self, frame: &AudioFrame) -> Result<(), PcmStreamError> {
        if frame.format() != self.format {
            return Err(PcmStreamError::FormatMismatch);
        }
        let mut bytes = Vec::with_capacity(std::mem::size_of_val(frame.samples()));
        for sample in frame.samples() {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.writer
            .write_all(&bytes)
            .map_err(|_| PcmStreamError::WriteFailed)
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

fn validate_hfp_format(format: AudioFormat) -> Result<(), PcmStreamError> {
    if matches!(
        format,
        AudioFormat::HFP_NARROWBAND | AudioFormat::HFP_WIDEBAND
    ) {
        Ok(())
    } else {
        Err(PcmStreamError::UnsupportedFormat)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PcmStreamError {
    #[error("PCM format is unsupported")]
    UnsupportedFormat,
    #[error("PCM input ended within a frame")]
    TruncatedFrame,
    #[error("PCM input could not be read")]
    ReadFailed,
    #[error("PCM output could not be written")]
    WriteFailed,
    #[error("PCM frame format does not match the stream")]
    FormatMismatch,
    #[error("PCM frame sequence is exhausted")]
    SequenceExhausted,
    #[error("PCM frame is invalid")]
    InvalidFrame,
}

/// Transport-neutral diagnostic bridge between live PCM frames and ACAP packets.
/// It owns only bounded in-memory jitter state and never logs or persists samples.
pub struct FramedPcmMediaBridge {
    format: AudioFormat,
    uplink: JitterBuffer,
}

impl FramedPcmMediaBridge {
    pub fn new(
        format: AudioFormat,
        uplink_capacity: usize,
        uplink_target_depth: usize,
    ) -> Result<Self, FramedPcmMediaError> {
        validate_hfp_format(format).map_err(|_| FramedPcmMediaError::UnsupportedFormat)?;
        let uplink = JitterBuffer::new(uplink_capacity, uplink_target_depth)
            .map_err(|_| FramedPcmMediaError::InvalidJitterConfiguration)?;
        Ok(Self { format, uplink })
    }

    pub fn encode_downlink(
        &self,
        frame: &AudioFrame,
        capture_time_micros: u64,
    ) -> Result<Vec<u8>, FramedPcmMediaError> {
        if frame.format() != self.format {
            return Err(FramedPcmMediaError::FormatMismatch);
        }
        AudioPacket::new(capture_time_micros, frame.clone())
            .encode()
            .map_err(|_| FramedPcmMediaError::InvalidPacket)
    }

    pub fn receive_uplink(&mut self, packet: &[u8]) -> Result<(), FramedPcmMediaError> {
        let packet = AudioPacket::decode(packet).map_err(|_| FramedPcmMediaError::InvalidPacket)?;
        if packet.frame().format() != self.format {
            return Err(FramedPcmMediaError::FormatMismatch);
        }
        self.uplink.insert(packet.frame().clone());
        Ok(())
    }

    /// Advances one uplink playout interval. Call at the active HFP frame rate.
    pub fn tick_uplink(&mut self) -> Option<AudioFrame> {
        self.uplink.tick()
    }

    #[must_use]
    pub fn uplink_summary(&self) -> JitterBufferSummary {
        self.uplink.summary()
    }
}

impl std::fmt::Debug for FramedPcmMediaBridge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FramedPcmMediaBridge")
            .field("format", &self.format)
            .field("uplink", &self.uplink.summary())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FramedPcmMediaError {
    #[error("media format is unsupported")]
    UnsupportedFormat,
    #[error("media jitter configuration is invalid")]
    InvalidJitterConfiguration,
    #[error("media packet format changed")]
    FormatMismatch,
    #[error("media packet is invalid")]
    InvalidPacket,
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

    pub fn reset(&self) -> Result<(), AudioQueueError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AudioQueueError::LockPoisoned)?;
        state.frames.clear();
        state.summary = AudioQueueSummary::default();
        Ok(())
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

    pub fn reset(&self) -> Result<(), AudioQueueError> {
        self.uplink.reset()?;
        self.downlink.reset()
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
    fn reset_discards_stale_frames_and_counters_between_calls() {
        let bridge = AudioBridge::new(4).unwrap();
        bridge.uplink.push(frame(1)).unwrap();
        bridge.downlink.push(frame(2)).unwrap();
        bridge.reset().unwrap();
        assert_eq!(bridge.summary().unwrap(), AudioBridgeSummary::default());
        assert!(bridge.uplink.pop().unwrap().is_none());
        assert!(bridge.downlink.pop().unwrap().is_none());
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
    fn live_bridge_helpers_move_pcm_in_both_directions_and_fill_underflow() {
        use std::io::Cursor;

        let format = AudioFormat::HFP_NARROWBAND;
        let source_frame = AudioFrame::new(
            0,
            format,
            (0..format.samples_per_channel)
                .map(|sample| i16::try_from(sample).unwrap())
                .collect(),
        )
        .unwrap();
        let mut source_bytes = Vec::new();
        for sample in source_frame.samples() {
            source_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let bridge = AudioBridge::new(2).unwrap();
        let mut capture = PcmFrameReader::new(Cursor::new(source_bytes), format).unwrap();
        assert!(capture_into_bridge(&mut capture, &bridge).unwrap());
        assert_eq!(bridge.downlink.pop().unwrap(), Some(source_frame));
        assert!(!capture_into_bridge(&mut capture, &bridge).unwrap());

        let uplink = frame(44);
        let wideband_bridge = AudioBridge::new(2).unwrap();
        wideband_bridge.uplink.push(uplink.clone()).unwrap();
        let silence = AudioFrame::new(
            0,
            AudioFormat::HFP_WIDEBAND,
            vec![0; usize::from(AudioFormat::HFP_WIDEBAND.samples_per_channel)],
        )
        .unwrap();
        let mut playback = PcmFrameWriter::new(Vec::new(), AudioFormat::HFP_WIDEBAND).unwrap();
        playback_from_bridge(&mut playback, &wideband_bridge, &silence).unwrap();
        playback_from_bridge(&mut playback, &wideband_bridge, &silence).unwrap();
        let output = playback.into_inner();
        let frame_bytes = usize::from(AudioFormat::HFP_WIDEBAND.samples_per_channel) * 2;
        assert_eq!(output.len(), frame_bytes * 2);
        assert!(output[frame_bytes..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn jitter_buffer_reorders_before_playout() {
        let mut buffer = JitterBuffer::new(4, 3).unwrap();
        buffer.insert(frame(2));
        buffer.insert(frame(1));
        assert!(buffer.tick().is_none());
        buffer.insert(frame(3));
        assert_eq!(buffer.tick().unwrap().sequence(), 1);
        assert_eq!(buffer.tick().unwrap().sequence(), 2);
        assert_eq!(buffer.tick().unwrap().sequence(), 3);
        assert_eq!(buffer.summary().emitted, 3);
    }

    #[test]
    fn jitter_buffer_counts_loss_duplicates_and_late_frames() {
        let mut buffer = JitterBuffer::new(4, 2).unwrap();
        buffer.insert(frame(10));
        buffer.insert(frame(12));
        buffer.insert(frame(12));
        assert_eq!(buffer.tick().unwrap().sequence(), 10);
        assert!(buffer.tick().is_none());
        buffer.insert(frame(9));
        assert_eq!(buffer.tick().unwrap().sequence(), 12);
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
        assert_eq!(buffer.tick().unwrap().sequence(), 5);
        assert_eq!(buffer.tick().unwrap().sequence(), 6);
    }

    #[test]
    fn jitter_tick_counts_empty_playout_underflow_but_not_prestart_polling() {
        let mut buffer = JitterBuffer::new(2, 1).unwrap();
        assert!(buffer.tick().is_none());
        assert_eq!(buffer.summary().missing, 0);
        buffer.insert(frame(20));
        assert_eq!(buffer.tick().unwrap().sequence(), 20);
        assert!(buffer.tick().is_none());
        assert_eq!(buffer.summary().missing, 1);
        buffer.insert(frame(21));
        assert_eq!(buffer.summary().late, 1);
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
        assert_eq!(
            ScoNodeLocator::new(FixtureDumpRunner {
                dump: "[]".to_owned()
            })
            .locate(),
            Err(ScoNodeError::Absent)
        );
        let duplicate = r#"[
              {"id":1,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"11","factory.name":"api.bluez5.sco.source"}}},
              {"id":2,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"12","factory.name":"api.bluez5.sco.source"}}},
              {"id":3,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"13","factory.name":"api.bluez5.sco.sink"}}}
            ]"#;
        assert_eq!(
            ScoNodeLocator::new(FixtureDumpRunner {
                dump: duplicate.to_owned()
            })
            .locate(),
            Err(ScoNodeError::Ambiguous)
        );
        assert_eq!(
            ScoNodeLocator::new(FixtureDumpRunner {
                dump: "private malformed payload".to_owned()
            })
            .locate(),
            Err(ScoNodeError::InvalidSnapshot)
        );
    }

    #[test]
    fn pipewire_audio_snapshot_maps_presence_and_absence_without_identifiers() {
        let active = PipeWireAudioStateBackend::new(FixtureDumpRunner {
            dump: r#"[
              {"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"71","factory.name":"api.bluez5.sco.source"}}},
              {"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":"72","factory.name":"api.bluez5.sco.sink"}}}
            ]"#
                .to_owned(),
        });
        assert_eq!(active.snapshot(), Ok(AudioTransportState::ScoActive));
        let inactive = PipeWireAudioStateBackend::new(FixtureDumpRunner {
            dump: "[]".to_owned(),
        });
        assert_eq!(inactive.snapshot(), Ok(AudioTransportState::Inactive));
        let invalid = PipeWireAudioStateBackend::new(FixtureDumpRunner {
            dump: "private malformed snapshot".to_owned(),
        });
        assert_eq!(invalid.snapshot(), Err(ScoNodeError::InvalidSnapshot));
    }

    #[test]
    fn sco_teardown_watchdog_is_monotonic_bounded_and_resets() {
        let start = Instant::now();
        let mut watchdog = ScoTeardownWatchdog::new(std::time::Duration::from_secs(10));

        let tearing_down = watchdog.observe(CallState::Idle, AudioTransportState::ScoActive, start);
        assert_eq!(tearing_down.state, AudioTransportState::ScoTearingDown);
        assert!(!tearing_down.stalled);
        let stalled = watchdog.observe(
            CallState::Idle,
            AudioTransportState::ScoActive,
            start + std::time::Duration::from_secs(10),
        );
        assert_eq!(stalled.state, AudioTransportState::Error);
        assert!(stalled.stalled);

        let recovered = watchdog.observe(
            CallState::Idle,
            AudioTransportState::Inactive,
            start + std::time::Duration::from_secs(11),
        );
        assert_eq!(recovered.state, AudioTransportState::Inactive);
        assert!(!recovered.stalled);
        let next_call = watchdog.observe(
            CallState::Active,
            AudioTransportState::ScoActive,
            start + std::time::Duration::from_secs(12),
        );
        assert_eq!(next_call.state, AudioTransportState::ScoActive);
        assert!(!next_call.stalled);
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

    #[test]
    fn pw_cat_session_transfers_each_pipe_at_most_once() {
        let nodes = ScoNodePair {
            source_serial: 701,
            sink_serial: 702,
        };
        let mut session = PwCatSession::start("true", nodes, AudioFormat::HFP_WIDEBAND).unwrap();
        let streams = session.take_frame_streams().unwrap();
        let (_downlink, _uplink) = streams.into_parts();
        assert!(matches!(
            session.take_frame_streams(),
            Err(PwCatStreamError::PipeUnavailable)
        ));
    }

    #[test]
    fn pcm_reader_reassembles_partial_reads_and_sequences_frames() {
        struct SmallReads {
            bytes: std::io::Cursor<Vec<u8>>,
        }
        impl Read for SmallReads {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let limit = output.len().min(3);
                self.bytes.read(&mut output[..limit])
            }
        }

        let sample_count = usize::from(AudioFormat::HFP_NARROWBAND.samples_per_channel);
        let mut bytes = Vec::new();
        for sample in (0..sample_count * 2).map(|value| value as i16 - 60) {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let mut reader = PcmFrameReader::new(
            SmallReads {
                bytes: std::io::Cursor::new(bytes),
            },
            AudioFormat::HFP_NARROWBAND,
        )
        .unwrap();
        let first = reader.read_frame().unwrap().unwrap();
        let second = reader.read_frame().unwrap().unwrap();
        assert_eq!(first.sequence(), 0);
        assert_eq!(second.sequence(), 1);
        assert_eq!(first.samples()[0], -60);
        assert_eq!(second.samples()[sample_count - 1], 59);
        assert!(reader.read_frame().unwrap().is_none());
    }

    #[test]
    fn pcm_reader_rejects_truncated_frames_without_disclosing_samples() {
        let private_sample_marker = 12_345_i16.to_le_bytes();
        let mut reader = PcmFrameReader::new(
            std::io::Cursor::new(private_sample_marker),
            AudioFormat::HFP_NARROWBAND,
        )
        .unwrap();
        let error = reader.read_frame().unwrap_err();
        assert_eq!(error, PcmStreamError::TruncatedFrame);
        assert!(!format!("{error:?}").contains("12345"));
    }

    #[test]
    fn pcm_writer_uses_little_endian_and_rejects_format_changes() {
        let samples = vec![-2_i16; usize::from(AudioFormat::HFP_NARROWBAND.samples_per_channel)];
        let narrow = AudioFrame::new(9, AudioFormat::HFP_NARROWBAND, samples).unwrap();
        let mut writer = PcmFrameWriter::new(Vec::new(), AudioFormat::HFP_NARROWBAND).unwrap();
        writer.write_frame(&narrow).unwrap();
        let bytes = writer.into_inner();
        assert_eq!(&bytes[..2], &(-2_i16).to_le_bytes());
        assert_eq!(bytes.len(), 120);

        let mut writer = PcmFrameWriter::new(Vec::new(), AudioFormat::HFP_NARROWBAND).unwrap();
        assert_eq!(
            writer.write_frame(&frame(10)),
            Err(PcmStreamError::FormatMismatch)
        );
        assert!(writer.into_inner().is_empty());
    }

    #[test]
    fn framed_media_bridge_encodes_downlink_without_sample_diagnostics() {
        let bridge = FramedPcmMediaBridge::new(AudioFormat::HFP_WIDEBAND, 4, 2).unwrap();
        let private_sample_marker = 12_345;
        let frame = AudioFrame::new(
            7,
            AudioFormat::HFP_WIDEBAND,
            vec![private_sample_marker; usize::from(AudioFormat::HFP_WIDEBAND.samples_per_channel)],
        )
        .unwrap();
        let encoded = bridge.encode_downlink(&frame, 99_000).unwrap();
        let decoded = AudioPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.capture_time_micros(), 99_000);
        assert_eq!(decoded.frame(), &frame);
        assert!(!format!("{bridge:?}").contains(&private_sample_marker.to_string()));
    }

    #[test]
    fn framed_media_bridge_reorders_uplink_and_rejects_bad_or_changed_packets() {
        let mut bridge = FramedPcmMediaBridge::new(AudioFormat::HFP_WIDEBAND, 4, 2).unwrap();
        for sequence in [2, 1] {
            let packet = AudioPacket::new(sequence * 7_500, frame(sequence))
                .encode()
                .unwrap();
            bridge.receive_uplink(&packet).unwrap();
        }
        assert_eq!(bridge.tick_uplink().unwrap().sequence(), 1);
        assert_eq!(bridge.tick_uplink().unwrap().sequence(), 2);
        assert_eq!(bridge.uplink_summary().emitted, 2);
        assert_eq!(
            bridge.receive_uplink(b"private malformed packet marker"),
            Err(FramedPcmMediaError::InvalidPacket)
        );

        let narrow = AudioFrame::new(
            3,
            AudioFormat::HFP_NARROWBAND,
            vec![0; usize::from(AudioFormat::HFP_NARROWBAND.samples_per_channel)],
        )
        .unwrap();
        let packet = AudioPacket::new(0, narrow).encode().unwrap();
        assert_eq!(
            bridge.receive_uplink(&packet),
            Err(FramedPcmMediaError::FormatMismatch)
        );
    }

    #[test]
    fn framed_media_bridge_rejects_invalid_configuration_and_downlink_format() {
        let unsupported = AudioFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            samples_per_channel: 360,
        };
        assert!(matches!(
            FramedPcmMediaBridge::new(unsupported, 4, 2),
            Err(FramedPcmMediaError::UnsupportedFormat)
        ));
        assert!(matches!(
            FramedPcmMediaBridge::new(AudioFormat::HFP_WIDEBAND, 0, 0),
            Err(FramedPcmMediaError::InvalidJitterConfiguration)
        ));
        let bridge = FramedPcmMediaBridge::new(AudioFormat::HFP_NARROWBAND, 4, 2).unwrap();
        assert_eq!(
            bridge.encode_downlink(&frame(1), 0),
            Err(FramedPcmMediaError::FormatMismatch)
        );
    }
}
