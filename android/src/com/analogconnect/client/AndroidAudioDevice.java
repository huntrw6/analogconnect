package com.analogconnect.client;

import android.content.Context;
import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioRecord;
import android.media.AudioTrack;
import android.media.MediaRecorder;
import android.media.audiofx.AcousticEchoCanceler;
import android.media.audiofx.NoiseSuppressor;

final class AndroidAudioDevice implements CallAudioPump.AudioIo {
    private final AudioDeviceConfig config;
    private final AudioManager audioManager;
    private final AudioRecord recorder;
    private final AudioTrack player;
    private final AcousticEchoCanceler echoCanceler;
    private final NoiseSuppressor noiseSuppressor;
    private int previousMode;
    private boolean previousSpeakerphone;
    private boolean speakerphone;
    private volatile boolean started;
    private volatile boolean closed;

    AndroidAudioDevice(Context context, int wireFormat) {
        config = AudioDeviceConfig.forWireFormat(wireFormat);
        audioManager = (AudioManager) context.getSystemService(Context.AUDIO_SERVICE);
        int inputMinimum = AudioRecord.getMinBufferSize(config.sampleRate,
                AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT);
        int outputMinimum = AudioTrack.getMinBufferSize(config.sampleRate,
                AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_16BIT);
        if (inputMinimum <= 0 || outputMinimum <= 0) {
            throw new IllegalStateException("Call audio format is unavailable");
        }
        int inputBuffer = Math.max(inputMinimum, config.preferredBufferBytes());
        int outputBuffer = Math.max(outputMinimum, config.preferredBufferBytes());
        AudioFormat inputFormat = new AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(config.sampleRate)
                .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
                .build();
        AudioFormat outputFormat = new AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(config.sampleRate)
                .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                .build();
        AudioRecord createdRecorder = null;
        AudioTrack createdPlayer = null;
        AcousticEchoCanceler createdEchoCanceler = null;
        NoiseSuppressor createdNoiseSuppressor = null;
        try {
            createdRecorder = new AudioRecord.Builder()
                    .setAudioSource(MediaRecorder.AudioSource.VOICE_COMMUNICATION)
                    .setAudioFormat(inputFormat)
                    .setBufferSizeInBytes(inputBuffer)
                    .build();
            createdPlayer = new AudioTrack.Builder()
                    .setAudioAttributes(new AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                            .build())
                    .setAudioFormat(outputFormat)
                    .setBufferSizeInBytes(outputBuffer)
                    .setTransferMode(AudioTrack.MODE_STREAM)
                    .build();
            if (createdRecorder.getState() != AudioRecord.STATE_INITIALIZED
                    || createdPlayer.getState() != AudioTrack.STATE_INITIALIZED) {
                throw new IllegalStateException("Call audio device initialization failed");
            }
            createdEchoCanceler = AcousticEchoCanceler.isAvailable()
                    ? AcousticEchoCanceler.create(createdRecorder.getAudioSessionId()) : null;
            createdNoiseSuppressor = NoiseSuppressor.isAvailable()
                    ? NoiseSuppressor.create(createdRecorder.getAudioSessionId()) : null;
            if (createdEchoCanceler != null) {
                createdEchoCanceler.setEnabled(true);
            }
            if (createdNoiseSuppressor != null) {
                createdNoiseSuppressor.setEnabled(true);
            }
        } catch (RuntimeException error) {
            releaseSafely(createdEchoCanceler, createdNoiseSuppressor,
                    createdRecorder, createdPlayer);
            throw new IllegalStateException("Call audio device initialization failed");
        }
        recorder = createdRecorder;
        player = createdPlayer;
        echoCanceler = createdEchoCanceler;
        noiseSuppressor = createdNoiseSuppressor;
    }

    public synchronized void start() {
        if (closed) {
            throw new IllegalStateException("Call audio device is closed");
        }
        if (started) {
            return;
        }
        previousMode = audioManager.getMode();
        previousSpeakerphone = audioManager.isSpeakerphoneOn();
        try {
            audioManager.setMode(AudioManager.MODE_IN_COMMUNICATION);
            audioManager.setSpeakerphoneOn(speakerphone);
            player.play();
            if (player.getPlayState() != AudioTrack.PLAYSTATE_PLAYING) {
                throw new IllegalStateException("Call speaker did not start");
            }
            recorder.startRecording();
            if (recorder.getRecordingState() != AudioRecord.RECORDSTATE_RECORDING) {
                throw new IllegalStateException("Call microphone did not start");
            }
            started = true;
        } catch (RuntimeException error) {
            stopHardwareSafely();
            restoreRoutingSafely();
            throw new IllegalStateException("Call audio device did not start");
        }
    }

    public AudioPacketCodec.Decoded readUplink(long sequence) {
        requireStarted();
        short[] samples = new short[config.samplesPerFrame];
        int read = recorder.read(samples, 0, samples.length, AudioRecord.READ_BLOCKING);
        if (read != samples.length) {
            throw new IllegalStateException("Call microphone frame was incomplete");
        }
        return new AudioPacketCodec.Decoded(
                config.wireFormat, sequence, System.nanoTime() / 1_000, samples);
    }

    public void writeDownlink(AudioPacketCodec.Decoded packet) {
        requireStarted();
        if (packet.format != config.wireFormat || packet.samples.length != config.samplesPerFrame) {
            throw new IllegalArgumentException("Call audio frame format changed");
        }
        int written = player.write(
                packet.samples, 0, packet.samples.length, AudioTrack.WRITE_BLOCKING);
        if (written != packet.samples.length) {
            throw new IllegalStateException("Call speaker frame was incomplete");
        }
    }

    public synchronized void stop() {
        if (!started) {
            return;
        }
        started = false;
        stopHardwareSafely();
        restoreRoutingSafely();
    }

    @Override
    public synchronized void setSpeakerphone(boolean enabled) {
        if (closed) {
            throw new IllegalStateException("Call audio device is closed");
        }
        speakerphone = enabled;
        if (started) {
            try {
                audioManager.setSpeakerphoneOn(enabled);
            } catch (RuntimeException error) {
                throw new IllegalStateException("Call audio routing failed");
            }
        }
    }

    private void restoreRouting() {
        audioManager.setSpeakerphoneOn(previousSpeakerphone);
        audioManager.setMode(previousMode);
    }

    private void restoreRoutingSafely() {
        try {
            restoreRouting();
        } catch (RuntimeException ignored) {
            // Never include vendor audio diagnostics in routine application output.
        }
    }

    private void stopHardwareSafely() {
        try {
            if (recorder.getRecordingState() == AudioRecord.RECORDSTATE_RECORDING) {
                recorder.stop();
            }
        } catch (RuntimeException ignored) {
            // Continue restoring the remaining resources.
        }
        try {
            if (player.getPlayState() == AudioTrack.PLAYSTATE_PLAYING) {
                player.pause();
            }
            player.flush();
        } catch (RuntimeException ignored) {
            // Continue restoring routing even if the platform player is unhealthy.
        }
    }

    private void requireStarted() {
        if (!started || closed) {
            throw new IllegalStateException("Call audio device is not started");
        }
    }

    private static void releaseSafely(AcousticEchoCanceler echo,
            NoiseSuppressor noise, AudioRecord audioRecord, AudioTrack audioTrack) {
        try {
            if (echo != null) {
                echo.release();
            }
        } catch (RuntimeException ignored) {
            // Continue releasing all other partially constructed resources.
        }
        try {
            if (noise != null) {
                noise.release();
            }
        } catch (RuntimeException ignored) {
            // Continue releasing all other partially constructed resources.
        }
        try {
            if (audioRecord != null) {
                audioRecord.release();
            }
        } catch (RuntimeException ignored) {
            // Continue releasing the player.
        }
        try {
            if (audioTrack != null) {
                audioTrack.release();
            }
        } catch (RuntimeException ignored) {
            // Construction is already failing; no diagnostic payload is retained.
        }
    }

    @Override
    public synchronized void close() {
        if (closed) {
            return;
        }
        stop();
        closed = true;
        releaseSafely(echoCanceler, noiseSuppressor, recorder, player);
    }
}
