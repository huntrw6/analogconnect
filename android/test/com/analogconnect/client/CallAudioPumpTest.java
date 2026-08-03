package com.analogconnect.client;

import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;

public final class CallAudioPumpTest {
    private static int tests;

    public static void main(String[] args) throws Exception {
        uplinkIsEncodedAndSequenced();
        downlinkIsDecodedReorderedAndPlayed();
        formatChangesFailClosed();
        diagnosticsContainNoSamplesOrTransportData();
        closeIsIdempotent();
        System.out.println("ANDROID_AUDIO_PUMP_TESTS=PASS tests=" + tests);
    }

    private static void uplinkIsEncodedAndSequenced() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        pump.captureOnce(9);
        AudioPacketCodec.Decoded decoded = AudioPacketCodec.decode(transport.sent.get(0));
        assertEquals(9, decoded.sequence);
        assertEquals(AudioPacketCodec.FORMAT_WIDEBAND, decoded.format);
        tests++;
    }

    private static void downlinkIsDecodedReorderedAndPlayed() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        transport.incoming.add(packet(11, AudioPacketCodec.FORMAT_WIDEBAND));
        transport.incoming.add(packet(10, AudioPacketCodec.FORMAT_WIDEBAND));
        transport.incoming.add(packet(12, AudioPacketCodec.FORMAT_WIDEBAND));
        pump.receiveOnce();
        pump.receiveOnce();
        pump.receiveOnce();
        pump.playoutOnce();
        pump.playoutOnce();
        pump.playoutOnce();
        assertEquals(3, audio.played.size());
        assertEquals(10, audio.played.get(0).sequence);
        assertEquals(11, audio.played.get(1).sequence);
        assertEquals(12, audio.played.get(2).sequence);
        assertEquals(3, pump.jitterSummary().emitted);
        tests++;
    }

    private static void formatChangesFailClosed() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        transport.incoming.add(packet(1, AudioPacketCodec.FORMAT_NARROWBAND));
        try {
            pump.receiveOnce();
            throw new AssertionError("expected format rejection");
        } catch (IOException expected) {
            assertFalse(expected.getMessage().contains("1"));
        }
        assertEquals(0, pump.jitterSummary().received);
        tests++;
    }

    private static void diagnosticsContainNoSamplesOrTransportData() {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        String diagnostic = pump.toString();
        assertFalse(diagnostic.contains("12345"));
        assertFalse(diagnostic.contains("token"));
        tests++;
    }

    private static void closeIsIdempotent() {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        pump.close();
        pump.close();
        assertEquals(1, transport.closeCalls);
        assertEquals(1, audio.stopCalls);
        assertEquals(1, audio.closeCalls);
        tests++;
    }

    private static byte[] packet(long sequence, int format) throws Exception {
        int samples = format == AudioPacketCodec.FORMAT_WIDEBAND ? 120 : 60;
        return AudioPacketCodec.encode(format, sequence, sequence * 7_500,
                new short[samples]);
    }

    private static final class FakeAudio implements CallAudioPump.AudioIo {
        private final int format;
        final List<AudioPacketCodec.Decoded> played = new ArrayList<>();
        int stopCalls;
        int closeCalls;

        FakeAudio(int format) {
            this.format = format;
        }

        @Override public void start() {}

        @Override public AudioPacketCodec.Decoded readUplink(long sequence) {
            int samples = format == AudioPacketCodec.FORMAT_WIDEBAND ? 120 : 60;
            short[] values = new short[samples];
            values[0] = 12345;
            return new AudioPacketCodec.Decoded(format, sequence, 99, values);
        }

        @Override public void writeDownlink(AudioPacketCodec.Decoded packet) {
            played.add(packet);
        }

        @Override public void stop() {
            stopCalls++;
        }

        @Override public void close() {
            closeCalls++;
        }
    }

    private static final class FakeTransport implements CallAudioPump.Transport {
        final ArrayDeque<byte[]> incoming = new ArrayDeque<>();
        final List<byte[]> sent = new ArrayList<>();
        int closeCalls;

        @Override public void sendBinary(byte[] packet) {
            sent.add(packet);
        }

        @Override public byte[] receiveBinary() throws IOException {
            byte[] packet = incoming.poll();
            if (packet == null) {
                throw new IOException("no synthetic packet");
            }
            return packet;
        }

        @Override public void close() {
            closeCalls++;
        }
    }

    private static void assertEquals(long expected, long actual) {
        if (expected != actual) {
            throw new AssertionError("expected " + expected + " but got " + actual);
        }
    }

    private static void assertFalse(boolean value) {
        if (value) {
            throw new AssertionError("expected false");
        }
    }
}
