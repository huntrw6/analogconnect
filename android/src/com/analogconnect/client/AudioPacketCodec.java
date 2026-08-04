package com.analogconnect.client;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

final class AudioPacketCodec {
    static final int MAX_BATCH_FRAMES = 4;
    static final int FORMAT_NARROWBAND = 1;
    static final int FORMAT_WIDEBAND = 2;
    private static final int HEADER_BYTES = 24;
    private static final byte[] MAGIC = new byte[] {'A', 'C', 'A', 'P'};

    private AudioPacketCodec() {}

    static byte[] encode(int format, long sequence, long captureTimeMicros, short[] samples)
            throws PacketException {
        int expectedSamples = expectedSamples(format);
        if (sequence < 0) {
            throw new PacketException("Audio sequence is outside the supported range");
        }
        if (samples == null || samples.length != expectedSamples) {
            throw new PacketException("Invalid audio payload");
        }
        ByteBuffer packet = ByteBuffer.allocate(HEADER_BYTES + expectedSamples * 2);
        packet.order(ByteOrder.BIG_ENDIAN);
        packet.put(MAGIC);
        packet.put((byte) 1);
        packet.put((byte) format);
        packet.putShort((short) 0);
        packet.putLong(sequence);
        packet.putLong(captureTimeMicros);
        packet.order(ByteOrder.LITTLE_ENDIAN);
        for (short sample : samples) {
            packet.putShort(sample);
        }
        return packet.array();
    }

    static Decoded decode(byte[] bytes) throws PacketException {
        if (bytes == null || bytes.length < HEADER_BYTES) {
            throw new PacketException("Invalid audio header");
        }
        ByteBuffer packet = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN);
        for (byte value : MAGIC) {
            if (packet.get() != value) {
                throw new PacketException("Invalid audio header");
            }
        }
        if (packet.get() != 1) {
            throw new PacketException("Unsupported audio version");
        }
        int format = packet.get() & 0xff;
        if (packet.getShort() != 0) {
            throw new PacketException("Invalid audio header");
        }
        int expectedSamples = expectedSamples(format);
        if (packet.remaining() != 16 + expectedSamples * 2) {
            throw new PacketException("Invalid audio payload");
        }
        long sequence = packet.getLong();
        if (sequence < 0) {
            throw new PacketException("Audio sequence is outside the supported range");
        }
        long captureTimeMicros = packet.getLong();
        packet.order(ByteOrder.LITTLE_ENDIAN);
        short[] samples = new short[expectedSamples];
        for (int index = 0; index < samples.length; index++) {
            samples[index] = packet.getShort();
        }
        return new Decoded(format, sequence, captureTimeMicros, samples);
    }

    static byte[] encodeBatch(Decoded[] frames) throws PacketException {
        validateBatch(frames);
        int packetBytes = HEADER_BYTES + expectedSamples(frames[0].format) * 2;
        byte[] batch = new byte[packetBytes * frames.length];
        int offset = 0;
        for (Decoded frame : frames) {
            byte[] encoded = encode(
                    frame.format, frame.sequence, frame.captureTimeMicros, frame.samples);
            System.arraycopy(encoded, 0, batch, offset, encoded.length);
            offset += encoded.length;
        }
        return batch;
    }

    static Decoded[] decodeBatch(byte[] bytes) throws PacketException {
        if (bytes == null || bytes.length < HEADER_BYTES + 60 * 2) {
            throw new PacketException("Invalid audio batch");
        }
        int offset = 0;
        Decoded[] decoded = new Decoded[MAX_BATCH_FRAMES];
        int count = 0;
        while (offset < bytes.length && count < MAX_BATCH_FRAMES) {
            if (bytes.length - offset < 6) {
                throw new PacketException("Invalid audio batch");
            }
            int format = bytes[offset + 5] & 0xff;
            int packetBytes = HEADER_BYTES + expectedSamples(format) * 2;
            if (offset + packetBytes > bytes.length) {
                throw new PacketException("Invalid audio batch");
            }
            byte[] packet = new byte[packetBytes];
            System.arraycopy(bytes, offset, packet, 0, packetBytes);
            decoded[count++] = decode(packet);
            offset += packetBytes;
        }
        if (offset != bytes.length || count == 0) {
            throw new PacketException("Invalid audio batch");
        }
        Decoded[] result = new Decoded[count];
        System.arraycopy(decoded, 0, result, 0, count);
        validateBatch(result);
        return result;
    }

    private static void validateBatch(Decoded[] frames) throws PacketException {
        if (frames == null || frames.length == 0 || frames.length > MAX_BATCH_FRAMES) {
            throw new PacketException("Invalid audio batch");
        }
        int format = frames[0] == null ? -1 : frames[0].format;
        long sequence = frames[0] == null ? -1 : frames[0].sequence;
        for (int index = 0; index < frames.length; index++) {
            Decoded frame = frames[index];
            if (frame == null || frame.format != format || frame.sequence != sequence + index) {
                throw new PacketException("Invalid audio batch");
            }
            expectedSamples(frame.format);
        }
    }

    private static int expectedSamples(int format) throws PacketException {
        if (format == FORMAT_NARROWBAND) {
            return 60;
        }
        if (format == FORMAT_WIDEBAND) {
            return 120;
        }
        throw new PacketException("Unsupported audio format");
    }

    static final class Decoded {
        final int format;
        final long sequence;
        final long captureTimeMicros;
        final short[] samples;

        Decoded(int format, long sequence, long captureTimeMicros, short[] samples) {
            this.format = format;
            this.sequence = sequence;
            this.captureTimeMicros = captureTimeMicros;
            this.samples = samples;
        }

        @Override
        public String toString() {
            return "DecodedAudioPacket{format=" + format + ", sequence=" + sequence
                    + ", captureTimeMicros=" + captureTimeMicros
                    + ", sampleCount=" + samples.length + "}";
        }
    }

    static final class PacketException extends Exception {
        PacketException(String message) {
            super(message);
        }
    }
}
