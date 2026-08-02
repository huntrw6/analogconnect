use std::time::Instant;

use analogconnect_core::{AudioFormat, AudioFrame};
use analogconnectd::audio::AudioBridge;

const FRAME_COUNT: u64 = 100_000;
const QUEUE_CAPACITY: usize = 32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = AudioBridge::new(QUEUE_CAPACITY)?;
    let format = AudioFormat::HFP_WIDEBAND;
    let sample_count = usize::from(format.samples_per_channel) * usize::from(format.channels);
    let started = Instant::now();

    for sequence in 0..FRAME_COUNT {
        let frame = AudioFrame::new(sequence, format, vec![0; sample_count])?;
        bridge.uplink.push(frame)?;
        let _ = bridge.uplink.pop()?;
    }

    let elapsed = started.elapsed();
    let frames_per_second = if elapsed.as_secs_f64() > 0.0 {
        FRAME_COUNT as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let realtime_requirement = 1_000_000.0 / format.frame_duration_micros() as f64;
    let realtime_multiple = frames_per_second / realtime_requirement;
    let summary = bridge.summary()?;

    println!(
        "AUDIO_BENCH=PASS frames={} fps={:.0} realtime_multiple={:.1} dropped={}",
        FRAME_COUNT, frames_per_second, realtime_multiple, summary.uplink.dropped
    );
    Ok(())
}
