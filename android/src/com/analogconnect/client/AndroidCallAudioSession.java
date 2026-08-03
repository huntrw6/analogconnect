package com.analogconnect.client;

import android.content.Context;
import android.os.SystemClock;

import java.io.IOException;
import java.security.GeneralSecurityException;

final class AndroidCallAudioSession implements AutoCloseable {
    private final CallAudioPump pump;

    private AndroidCallAudioSession(CallAudioPump pump) {
        this.pump = pump;
    }

    static AndroidCallAudioSession connect(Context context, String endpoint, String pin,
            String tlsName, MediaSessionCredentials credentials, int wireFormat)
            throws IOException, GeneralSecurityException {
        if (credentials == null || credentials.isExpired(SystemClock.elapsedRealtime())) {
            throw new IOException("Media session credentials are missing or expired");
        }
        MediaWebSocket transport = MediaWebSocket.connect(
                endpoint, pin, tlsName, credentials);
        AndroidAudioDevice audio = null;
        try {
            audio = new AndroidAudioDevice(context, wireFormat);
            return new AndroidCallAudioSession(new CallAudioPump(audio, transport, wireFormat));
        } catch (RuntimeException error) {
            if (audio != null) {
                audio.close();
            }
            transport.close();
            throw new IllegalStateException("Call audio initialization failed");
        }
    }

    void start() {
        pump.start();
    }

    String errorCode() {
        return pump.errorCode();
    }

    AudioJitterBuffer.Summary jitterSummary() {
        return pump.jitterSummary();
    }

    @Override
    public void close() {
        pump.close();
    }

    @Override
    public String toString() {
        return "AndroidCallAudioSession{pump=" + pump + "}";
    }
}
