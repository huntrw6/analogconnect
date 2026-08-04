package com.analogconnect.client;

import java.util.Map;
import java.util.TreeMap;

final class AudioJitterBuffer {
    private final int capacity;
    private final int targetDepth;
    private final TreeMap<Long, AudioPacketCodec.Decoded> frames = new TreeMap<>();
    private Long nextSequence;
    private boolean started;
    private long received;
    private long emitted;
    private long duplicate;
    private long late;
    private long missing;
    private long overflow;

    AudioJitterBuffer(int capacity, int targetDepth) {
        if (capacity <= 0 || targetDepth <= 0 || targetDepth > capacity) {
            throw new IllegalArgumentException("Invalid jitter buffer capacity");
        }
        this.capacity = capacity;
        this.targetDepth = targetDepth;
    }

    synchronized void insert(AudioPacketCodec.Decoded packet) {
        received++;
        long sequence = packet.sequence;
        if (nextSequence != null && sequence < nextSequence) {
            late++;
            return;
        }
        if (frames.containsKey(sequence)) {
            duplicate++;
            return;
        }
        if (frames.size() == capacity) {
            overflow++;
            Map.Entry<Long, AudioPacketCodec.Decoded> furthest = frames.lastEntry();
            if (sequence < furthest.getKey()) {
                frames.remove(furthest.getKey());
            } else {
                frames.pollFirstEntry();
                frames.put(sequence, packet);
                if (started && !frames.isEmpty()
                        && nextSequence != null && nextSequence < frames.firstKey()) {
                    nextSequence = frames.firstKey();
                }
                return;
            }
        }
        frames.put(sequence, packet);
    }

    // Call exactly once per negotiated 7.5 ms frame after playout begins.
    synchronized AudioPacketCodec.Decoded tick() {
        if (!started) {
            if (frames.size() < targetDepth) {
                return null;
            }
            nextSequence = frames.firstKey();
            started = true;
        }
        if (nextSequence == null) {
            return null;
        }
        long sequence = nextSequence;
        AudioPacketCodec.Decoded packet = frames.remove(sequence);
        if (packet == null) {
            missing++;
            // An entirely empty queue usually means clock drift or a late network burst.
            // Hold the expected sequence so its audio is not discarded when it arrives.
            if (frames.isEmpty()) {
                return null;
            }
            nextSequence = sequence == Long.MAX_VALUE ? null : sequence + 1;
            return null;
        }
        nextSequence = sequence == Long.MAX_VALUE ? null : sequence + 1;
        emitted++;
        return packet;
    }

    synchronized boolean hasStarted() {
        return started;
    }

    synchronized AudioPacketCodec.Decoded peekNext() {
        return nextSequence == null ? null : frames.get(nextSequence);
    }

    synchronized Summary summary() {
        return new Summary(frames.size(), received, emitted, duplicate, late, missing, overflow);
    }

    static final class Summary {
        final int depth;
        final long received;
        final long emitted;
        final long duplicate;
        final long late;
        final long missing;
        final long overflow;

        Summary(int depth, long received, long emitted, long duplicate, long late,
                long missing, long overflow) {
            this.depth = depth;
            this.received = received;
            this.emitted = emitted;
            this.duplicate = duplicate;
            this.late = late;
            this.missing = missing;
            this.overflow = overflow;
        }

        @Override
        public String toString() {
            return "AudioJitterSummary{depth=" + depth + ", received=" + received
                    + ", emitted=" + emitted + ", duplicate=" + duplicate + ", late=" + late
                    + ", missing=" + missing + ", overflow=" + overflow + "}";
        }
    }
}
