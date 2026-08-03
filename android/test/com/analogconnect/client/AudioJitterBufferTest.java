package com.analogconnect.client;

public final class AudioJitterBufferTest {
    public static void main(String[] args) throws Exception {
        reordersBeforePlayout();
        countsLossDuplicatesAndLatePackets();
        boundsFutureLatency();
        countsEmptyPlayoutUnderflowButNotPrestartPolling();
        rejectsInvalidCapacity();
        System.out.println("ANDROID_JITTER_TESTS=PASS tests=5");
    }

    private static AudioPacketCodec.Decoded packet(long sequence) throws Exception {
        return AudioPacketCodec.decode(AudioPacketCodec.encode(
                AudioPacketCodec.FORMAT_WIDEBAND, sequence, sequence * 7_500, new short[120]));
    }

    private static void reordersBeforePlayout() throws Exception {
        AudioJitterBuffer buffer = new AudioJitterBuffer(4, 3);
        buffer.insert(packet(2));
        buffer.insert(packet(1));
        assertTrue(buffer.tick() == null);
        buffer.insert(packet(3));
        assertTrue(buffer.tick().sequence == 1);
        assertTrue(buffer.tick().sequence == 2);
        assertTrue(buffer.tick().sequence == 3);
    }

    private static void countsLossDuplicatesAndLatePackets() throws Exception {
        AudioJitterBuffer buffer = new AudioJitterBuffer(4, 2);
        buffer.insert(packet(10));
        buffer.insert(packet(12));
        buffer.insert(packet(12));
        assertTrue(buffer.tick().sequence == 10);
        assertTrue(buffer.tick() == null);
        buffer.insert(packet(9));
        assertTrue(buffer.tick().sequence == 12);
        AudioJitterBuffer.Summary summary = buffer.summary();
        assertTrue(summary.missing == 1);
        assertTrue(summary.duplicate == 1);
        assertTrue(summary.late == 1);
        assertTrue(!summary.toString().contains("samples"));
    }

    private static void boundsFutureLatency() throws Exception {
        AudioJitterBuffer buffer = new AudioJitterBuffer(2, 1);
        buffer.insert(packet(5));
        buffer.insert(packet(7));
        buffer.insert(packet(6));
        assertTrue(buffer.summary().overflow == 1);
        assertTrue(buffer.tick().sequence == 5);
        assertTrue(buffer.tick().sequence == 6);
    }

    private static void countsEmptyPlayoutUnderflowButNotPrestartPolling() throws Exception {
        AudioJitterBuffer buffer = new AudioJitterBuffer(2, 1);
        assertTrue(buffer.tick() == null);
        assertTrue(buffer.summary().missing == 0);
        buffer.insert(packet(20));
        assertTrue(buffer.tick().sequence == 20);
        assertTrue(buffer.tick() == null);
        assertTrue(buffer.summary().missing == 1);
        buffer.insert(packet(21));
        assertTrue(buffer.summary().late == 1);
    }

    private static void rejectsInvalidCapacity() {
        try {
            new AudioJitterBuffer(1, 2);
            throw new AssertionError("expected capacity rejection");
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }

    private static void assertTrue(boolean condition) {
        if (!condition) {
            throw new AssertionError("jitter buffer assertion failed");
        }
    }
}
