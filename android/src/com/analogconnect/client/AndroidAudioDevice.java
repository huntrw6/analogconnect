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

final class AndroidAudioDevice implements AutoCloseable {
    private final AudioDeviceConfig config;
    private final AudioManager audioManager;
    private final AudioRecord recorder;
    private final AudioTrack player;
    private final AcousticEchoCanceler echoCanceler;
    private final NoiseSuppressor noiseSuppressor;
    private int previousMode;
    private boolean previousSpeakerphone;
    private boolean started;

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
        recorder = new AudioRecord.Builder()
                .setAudioSource(MediaRecorder.AudioSource.VOICE_COMMUNICATION)
                .setAudioFormat(inputFormat)
                .setBufferSizeInBytes(inputBuffer)
                .build();
        player = new AudioTrack.Builder()
                .setAudioAttributes(new AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                        .build())
                .setAudioFormat(outputFormat)
                .setBufferSizeInBytes(outputBuffer)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .build();
        if (recorder.getState() != AudioRecord.STATE_INITIALIZED
                || player.getState() != AudioTrack.STATE_INITIALIZED) {
            recorder.release();
            player.release();
            throw new IllegalStateException("Call audio device initialization failed");
        }
        echoCanceler = AcousticEchoCanceler.isAvailable()
                ? AcousticEchoCanceler.create(recorder.getAudioSessionId()) : null;
        noiseSuppressor = NoiseSuppressor.isAvailable()
                ? NoiseSuppressor.create(recorder.getAudioSessionId()) : null;
        if (echoCanceler != null) {
            echoCanceler.setEnabled(true);
        }
        if (noiseSuppressor != null) {
            noiseSuppressor.setEnabled(true);
        }
    }

    void start() {
        if (started) {
            return;
        }
        previousMode = audioManager.getMode();
        previousSpeakerphone = audioManager.isSpeakerphoneOn();
        audioManager.setMode(AudioManager.MODE_IN_COMMUNICATION);
        audioManager.setSpeakerphoneOn(false);
        player.play();
        recorder.startRecording();
        if (recorder.getRecordingState() != AudioRecord.RECORDSTATE_RECORDING) {
            player.stop();
            restoreRouting();
            throw new IllegalStateException("Call microphone did not start");
        }
        started = true;
    }

    AudioPacketCodec.Decoded readUplink(long sequence) {
        requireStarted();
        short[] samples = new short[config.samplesPerFrame];
        int read = recorder.read(samples, 0, samples.length, AudioRecord.READ_BLOCKING);
        if (read != samples.length) {
            throw new IllegalStateException("Call microphone frame was incomplete");
        }
        return new AudioPacketCodec.Decoded(
                config.wireFormat, sequence, System.nanoTime() / 1_000, samples);
    }

    void writeDownlink(AudioPacketCodec.Decoded packet) {
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

    void stop() {
        if (!started) {
            return;
        }
        recorder.stop();
        player.pause();
        player.flush();
        restoreRouting();
        started = false;
    }

    private void restoreRouting() {
        audioManager.setSpeakerphoneOn(previousSpeakerphone);
        audioManager.setMode(previousMode);
    }

    private void requireStarted() {
        if (!started) {
            throw new IllegalStateException("Call audio device is not started");
        }
    }

    @Override
    public void close() {
        stop();
        if (echoCanceler != null) {
            echoCanceler.release();
        }
        if (noiseSuppressor != null) {
            noiseSuppressor.release();
        }
        recorder.release();
        player.release();
    }
}
