package com.analogconnect.client;

import java.io.IOException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.locks.LockSupport;

final class CallAudioPump implements AutoCloseable {
    private static final long FRAME_NANOS = 7_500_000L;
    private static final long PACING_STEP_NANOS = 15_000L;
    private static final long MAX_SLOW_PACING_NANOS = 150_000L;
    private static final long MAX_FAST_PACING_NANOS = -300_000L;
    private static final int TRIM_HIGH_WATER_DEPTH = 12;
    private static final int MIN_FRAMES_BETWEEN_TRIMS = 32;
    private static final int JITTER_CAPACITY = 64;
    private static final int JITTER_TARGET_DEPTH = 4;

    interface AudioIo extends AutoCloseable {
        void start();
        AudioPacketCodec.Decoded readUplink(long sequence);
        void writeDownlink(AudioPacketCodec.Decoded packet);
        void setSpeakerphone(boolean enabled);
        void stop();
        @Override void close();
    }

    interface Transport extends AutoCloseable {
        void sendBinary(byte[] packet) throws IOException;
        byte[] receiveBinary() throws IOException;
        @Override void close();
    }

    private final AudioIo audio;
    private final Transport transport;
    private final int wireFormat;
    private final AudioJitterBuffer jitter =
            new AudioJitterBuffer(JITTER_CAPACITY, JITTER_TARGET_DEPTH);
    private final AtomicBoolean running = new AtomicBoolean();
    private final AtomicBoolean closed = new AtomicBoolean();
    private final AtomicReference<String> errorCode = new AtomicReference<>();
    private final AtomicLong concealedFrames = new AtomicLong();
    private short lastDownlinkSample;
    private boolean playedDownlink;
    private boolean previousFrameConcealed;
    private long pacingAdjustmentNanos;
    private long trimmedFrames;
    private int framesSinceTrim = MIN_FRAMES_BETWEEN_TRIMS;
    private Thread uplinkThread;
    private Thread receiveThread;
    private Thread playoutThread;

    CallAudioPump(AudioIo audio, Transport transport, int wireFormat) {
        AudioDeviceConfig.forWireFormat(wireFormat);
        if (audio == null || transport == null) {
            throw new IllegalArgumentException("Call audio boundary is missing");
        }
        this.audio = audio;
        this.transport = transport;
        this.wireFormat = wireFormat;
    }

    synchronized void start() {
        if (closed.get()) {
            throw new IllegalStateException("Call audio session is closed");
        }
        if (running.get()) {
            return;
        }
        if (uplinkThread != null) {
            throw new IllegalStateException("Call audio session cannot restart");
        }
        audio.start();
        running.set(true);
        uplinkThread = worker("AnalogConnect-uplink", new Runnable() {
            @Override public void run() {
                long sequence = 0;
                while (running.get()) {
                    try {
                        captureBatch(sequence);
                        if (sequence > Long.MAX_VALUE - AudioPacketCodec.MAX_BATCH_FRAMES) {
                            fail("audio_sequence_exhausted");
                            return;
                        }
                        sequence += AudioPacketCodec.MAX_BATCH_FRAMES;
                    } catch (IOException | RuntimeException error) {
                        fail("audio_uplink_failed");
                    }
                }
            }
        });
        receiveThread = worker("AnalogConnect-downlink-receive", new Runnable() {
            @Override public void run() {
                while (running.get()) {
                    try {
                        receiveOnce();
                    } catch (IOException | AudioPacketCodec.PacketException
                            | RuntimeException error) {
                        fail("audio_downlink_failed");
                    }
                }
            }
        });
        playoutThread = worker("AnalogConnect-downlink-playout", new Runnable() {
            @Override public void run() {
                long deadline = System.nanoTime();
                while (running.get()) {
                    deadline += FRAME_NANOS + pacingAdjustmentNanos;
                    long wait = deadline - System.nanoTime();
                    if (wait > 0) {
                        LockSupport.parkNanos(wait);
                    } else if (wait < -FRAME_NANOS) {
                        deadline = System.nanoTime();
                    }
                    try {
                        playoutOnce();
                    } catch (RuntimeException error) {
                        fail("audio_playout_failed");
                    }
                }
            }
        });
        uplinkThread.start();
        receiveThread.start();
        playoutThread.start();
    }

    void captureOnce(long sequence) throws IOException {
        AudioPacketCodec.Decoded packet = audio.readUplink(sequence);
        if (packet.format != wireFormat || packet.sequence != sequence) {
            throw new IOException("Call microphone format changed");
        }
        try {
            transport.sendBinary(AudioPacketCodec.encode(packet.format, packet.sequence,
                    packet.captureTimeMicros, packet.samples));
        } catch (AudioPacketCodec.PacketException error) {
            throw new IOException("Call microphone packet is invalid");
        }
    }

    void captureBatch(long firstSequence) throws IOException {
        AudioPacketCodec.Decoded[] frames =
                new AudioPacketCodec.Decoded[AudioPacketCodec.MAX_BATCH_FRAMES];
        for (int index = 0; index < frames.length; index++) {
            long sequence = firstSequence + index;
            frames[index] = audio.readUplink(sequence);
            if (frames[index].format != wireFormat || frames[index].sequence != sequence) {
                throw new IOException("Call microphone format changed");
            }
        }
        try {
            transport.sendBinary(AudioPacketCodec.encodeBatch(frames));
        } catch (AudioPacketCodec.PacketException error) {
            throw new IOException("Call microphone packet is invalid");
        }
    }

    void receiveOnce() throws IOException, AudioPacketCodec.PacketException {
        AudioPacketCodec.Decoded[] packets =
                AudioPacketCodec.decodeBatch(transport.receiveBinary());
        for (AudioPacketCodec.Decoded packet : packets) {
            if (packet.format != wireFormat) {
                throw new IOException("Call speaker format changed");
            }
        }
        for (AudioPacketCodec.Decoded packet : packets) {
            jitter.insert(packet);
        }
    }

    void playoutOnce() {
        AudioPacketCodec.Decoded packet = jitter.tick();
        if (packet != null) {
            framesSinceTrim++;
            if (jitter.summary().depth > TRIM_HIGH_WATER_DEPTH
                    && framesSinceTrim >= MIN_FRAMES_BETWEEN_TRIMS) {
                AudioPacketCodec.Decoded newer = jitter.tick();
                if (newer != null) {
                    packet = newer;
                    trimmedFrames++;
                    framesSinceTrim = 0;
                    previousFrameConcealed = true;
                }
            }
            if (previousFrameConcealed) {
                packet = smoothRecovery(packet);
            }
            audio.writeDownlink(packet);
            lastDownlinkSample = packet.samples[packet.samples.length - 1];
            playedDownlink = true;
            previousFrameConcealed = false;
            int depth = jitter.summary().depth;
            if (depth > JITTER_TARGET_DEPTH + 1) {
                pacingAdjustmentNanos = Math.max(MAX_FAST_PACING_NANOS,
                        pacingAdjustmentNanos - PACING_STEP_NANOS);
            } else if (depth >= JITTER_TARGET_DEPTH && pacingAdjustmentNanos > 0) {
                pacingAdjustmentNanos = Math.max(0,
                        pacingAdjustmentNanos - PACING_STEP_NANOS);
            } else if (depth < JITTER_TARGET_DEPTH - 1 && pacingAdjustmentNanos < 0) {
                pacingAdjustmentNanos = Math.min(0,
                        pacingAdjustmentNanos + PACING_STEP_NANOS);
            }
        } else if (jitter.hasStarted()) {
            audio.writeDownlink(concealMissingFrame());
            concealedFrames.incrementAndGet();
            previousFrameConcealed = true;
            pacingAdjustmentNanos = Math.min(MAX_SLOW_PACING_NANOS,
                    pacingAdjustmentNanos + PACING_STEP_NANOS);
        }
    }

    private AudioPacketCodec.Decoded smoothRecovery(AudioPacketCodec.Decoded packet) {
        short[] samples = packet.samples.clone();
        int transitionSamples = Math.min(samples.length, wireFormat ==
                AudioPacketCodec.FORMAT_WIDEBAND ? 24 : 12);
        for (int index = 0; index < transitionSamples; index++) {
            long oldWeight = transitionSamples - index - 1L;
            long newWeight = index + 1L;
            samples[index] = (short) ((lastDownlinkSample * oldWeight
                    + samples[index] * newWeight) / transitionSamples);
        }
        return new AudioPacketCodec.Decoded(packet.format, packet.sequence,
                packet.captureTimeMicros, samples);
    }

    private AudioPacketCodec.Decoded concealMissingFrame() {
        AudioPacketCodec.Decoded next = jitter.peekNext();
        short endSample = next == null ? 0 : next.samples[0];
        short startSample = playedDownlink ? lastDownlinkSample : endSample;
        int sampleCount = AudioDeviceConfig.forWireFormat(wireFormat).samplesPerFrame;
        short[] samples = new short[sampleCount];
        for (int index = 0; index < samples.length; index++) {
            long delta = (long) (endSample - startSample) * (index + 1);
            samples[index] = (short) (startSample + delta / samples.length);
        }
        lastDownlinkSample = endSample;
        playedDownlink = true;
        return new AudioPacketCodec.Decoded(wireFormat, 0, 0, samples);
    }

    String errorCode() {
        return errorCode.get();
    }

    AudioJitterBuffer.Summary jitterSummary() {
        return jitter.summary();
    }

    long concealedFrames() {
        return concealedFrames.get();
    }

    long pacingAdjustmentNanos() {
        return pacingAdjustmentNanos;
    }

    long trimmedFrames() {
        return trimmedFrames;
    }

    void setSpeakerphone(boolean enabled) {
        audio.setSpeakerphone(enabled);
    }

    private Thread worker(String name, Runnable runnable) {
        Thread thread = new Thread(runnable, name);
        thread.setDaemon(true);
        return thread;
    }

    private void fail(String code) {
        if (errorCode.compareAndSet(null, code)) {
            running.set(false);
            transport.close();
            audio.stop();
        }
    }

    @Override
    public synchronized void close() {
        if (!closed.compareAndSet(false, true)) {
            return;
        }
        running.set(false);
        transport.close();
        audio.stop();
        interruptAndJoin(uplinkThread);
        interruptAndJoin(receiveThread);
        interruptAndJoin(playoutThread);
        audio.close();
    }

    private static void interruptAndJoin(Thread thread) {
        if (thread == null || thread == Thread.currentThread()) {
            return;
        }
        thread.interrupt();
        try {
            thread.join(1_000);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        }
    }

    @Override
    public String toString() {
        return "CallAudioPump{running=" + running.get() + ", errorCode=" + errorCode.get()
                + ", concealed=" + concealedFrames.get() + ", trimmed=" + trimmedFrames
                + ", jitter=" + jitter.summary() + "}";
    }
}
