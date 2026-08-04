package com.analogconnect.client;

import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;

public final class CallAudioPumpTest {
    private static int tests;

    public static void main(String[] args) throws Exception {
        uplinkIsEncodedAndSequenced();
        uplinkBatchesFourFrames();
        downlinkIsDecodedReorderedAndPlayed();
        missingDownlinkFrameIsSmoothlyConcealed();
        emptyQueueHoldsExpectedFrameAndAdaptsPacing();
        sustainedBacklogIsTrimmedGradually();
        formatChangesFailClosed();
        diagnosticsContainNoSamplesOrTransportData();
        closeIsIdempotent();
        speakerphoneRoutingDelegatesToAudioBoundary();
        System.out.println("ANDROID_AUDIO_PUMP_TESTS=PASS tests=" + tests);
    }

    private static void uplinkBatchesFourFrames() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        pump.captureBatch(20);
        AudioPacketCodec.Decoded[] decoded =
                AudioPacketCodec.decodeBatch(transport.sent.get(0));
        assertEquals(4, decoded.length);
        assertEquals(20, decoded[0].sequence);
        assertEquals(23, decoded[3].sequence);
        tests++;
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
        for (long sequence = 12; sequence < 18; sequence++) {
            transport.incoming.add(packet(sequence, AudioPacketCodec.FORMAT_WIDEBAND));
        }
        for (int index = 0; index < 8; index++) {
            pump.receiveOnce();
        }
        for (int index = 0; index < 8; index++) {
            pump.playoutOnce();
        }
        assertEquals(8, audio.played.size());
        assertEquals(10, audio.played.get(0).sequence);
        assertEquals(11, audio.played.get(1).sequence);
        assertEquals(17, audio.played.get(7).sequence);
        assertEquals(8, pump.jitterSummary().emitted);
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

    private static void sustainedBacklogIsTrimmedGradually() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        for (long sequence = 100; sequence < 120; sequence++) {
            transport.incoming.add(packetWithSample(sequence, (short) sequence));
            pump.receiveOnce();
        }
        pump.playoutOnce();
        assertEquals(1, audio.played.size());
        assertEquals(101, audio.played.get(0).sequence);
        assertEquals(1, pump.trimmedFrames());
        assertEquals(18, pump.jitterSummary().depth);
        tests++;
    }

    private static void emptyQueueHoldsExpectedFrameAndAdaptsPacing() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        for (long sequence = 20; sequence < 24; sequence++) {
            transport.incoming.add(packet(sequence, AudioPacketCodec.FORMAT_WIDEBAND));
            pump.receiveOnce();
        }
        for (int index = 0; index < 5; index++) {
            pump.playoutOnce();
        }
        assertEquals(5, audio.played.size());
        assertTrue(pump.pacingAdjustmentNanos() > 0);
        transport.incoming.add(packet(24, AudioPacketCodec.FORMAT_WIDEBAND));
        pump.receiveOnce();
        pump.playoutOnce();
        assertEquals(24, audio.played.get(5).sequence);
        assertEquals(0, pump.jitterSummary().late);
        tests++;
    }

    private static void missingDownlinkFrameIsSmoothlyConcealed() throws Exception {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        FakeTransport transport = new FakeTransport();
        CallAudioPump pump = new CallAudioPump(
                audio, transport, AudioPacketCodec.FORMAT_WIDEBAND);
        transport.incoming.add(packetWithSample(10, (short) 1_000));
        transport.incoming.add(packetWithSample(11, (short) 2_000));
        transport.incoming.add(packetWithSample(14, (short) 5_000));
        transport.incoming.add(packetWithSample(15, (short) 6_000));
        for (int index = 0; index < 4; index++) {
            pump.receiveOnce();
        }
        pump.playoutOnce();
        pump.playoutOnce();
        pump.playoutOnce();
        transport.incoming.add(packetWithSample(13, (short) 4_000));
        pump.receiveOnce();
        pump.playoutOnce();
        assertEquals(4, audio.played.size());
        AudioPacketCodec.Decoded concealed = audio.played.get(2);
        assertEquals(1, pump.concealedFrames());
        assertEquals(1, pump.jitterSummary().missing);
        assertTrue(concealed.samples[0] > 0);
        assertTrue(concealed.samples[0] < 2_000);
        assertEquals(0, concealed.samples[concealed.samples.length - 1]);
        assertEquals(13, audio.played.get(3).sequence);
        assertTrue(audio.played.get(3).samples[0] < 4_000);
        assertEquals(4_000, audio.played.get(3).samples[23]);
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

    private static void speakerphoneRoutingDelegatesToAudioBoundary() {
        FakeAudio audio = new FakeAudio(AudioPacketCodec.FORMAT_WIDEBAND);
        CallAudioPump pump = new CallAudioPump(
                audio, new FakeTransport(), AudioPacketCodec.FORMAT_WIDEBAND);
        pump.setSpeakerphone(true);
        assertTrue(audio.speakerphone);
        pump.setSpeakerphone(false);
        assertFalse(audio.speakerphone);
        tests++;
    }

    private static byte[] packet(long sequence, int format) throws Exception {
        int samples = format == AudioPacketCodec.FORMAT_WIDEBAND ? 120 : 60;
        return AudioPacketCodec.encode(format, sequence, sequence * 7_500,
                new short[samples]);
    }

    private static byte[] packetWithSample(long sequence, short sample) throws Exception {
        short[] samples = new short[120];
        java.util.Arrays.fill(samples, sample);
        return AudioPacketCodec.encode(AudioPacketCodec.FORMAT_WIDEBAND, sequence,
                sequence * 7_500, samples);
    }

    private static final class FakeAudio implements CallAudioPump.AudioIo {
        private final int format;
        final List<AudioPacketCodec.Decoded> played = new ArrayList<>();
        int stopCalls;
        int closeCalls;
        boolean speakerphone;

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

        @Override public void setSpeakerphone(boolean enabled) {
            speakerphone = enabled;
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

    private static void assertTrue(boolean value) {
        if (!value) {
            throw new AssertionError("expected true");
        }
    }
}
