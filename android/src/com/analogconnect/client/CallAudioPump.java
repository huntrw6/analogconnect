package com.analogconnect.client;

import java.io.IOException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.locks.LockSupport;

final class CallAudioPump implements AutoCloseable {
    private static final long FRAME_NANOS = 7_500_000L;
    private static final int JITTER_CAPACITY = 8;
    private static final int JITTER_TARGET_DEPTH = 3;

    interface AudioIo extends AutoCloseable {
        void start();
        AudioPacketCodec.Decoded readUplink(long sequence);
        void writeDownlink(AudioPacketCodec.Decoded packet);
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
                        captureOnce(sequence);
                        if (sequence == Long.MAX_VALUE) {
                            fail("audio_sequence_exhausted");
                            return;
                        }
                        sequence++;
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
                    deadline += FRAME_NANOS;
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

    void receiveOnce() throws IOException, AudioPacketCodec.PacketException {
        AudioPacketCodec.Decoded packet = AudioPacketCodec.decode(transport.receiveBinary());
        if (packet.format != wireFormat) {
            throw new IOException("Call speaker format changed");
        }
        jitter.insert(packet);
    }

    void playoutOnce() {
        AudioPacketCodec.Decoded packet = jitter.tick();
        if (packet != null) {
            audio.writeDownlink(packet);
        }
    }

    String errorCode() {
        return errorCode.get();
    }

    AudioJitterBuffer.Summary jitterSummary() {
        return jitter.summary();
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
                + ", jitter=" + jitter.summary() + "}";
    }
}
