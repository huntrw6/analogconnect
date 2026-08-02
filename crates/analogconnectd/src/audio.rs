use std::{collections::VecDeque, sync::Mutex, time::Instant};

use analogconnect_core::AudioFrame;
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
    use analogconnect_core::AudioFormat;

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
}
