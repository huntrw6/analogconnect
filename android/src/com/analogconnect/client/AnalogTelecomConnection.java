package com.analogconnect.client;

import android.content.Context;
import android.telecom.Connection;
import android.telecom.CallAudioState;
import android.telecom.DisconnectCause;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

final class AnalogTelecomConnection extends Connection {
    private static final int POLL_INTERVAL_MS = 500;
    private static final int STARTUP_POLLS = 20;
    private final Context context;
    private final ExecutorService monitorExecutor = Executors.newSingleThreadExecutor();
    private final ExecutorService commandExecutor = Executors.newSingleThreadExecutor();
    private final AtomicBoolean closed = new AtomicBoolean();
    private volatile AndroidCallAudioSession audioSession;
    private volatile boolean speakerphone;

    AnalogTelecomConnection(Context context) {
        this.context = context.getApplicationContext();
        setAudioModeIsVoip(true);
    }

    void dial(String target) {
        setDialing();
        monitorExecutor.execute(new Runnable() {
            @Override public void run() {
                try {
                    EnrollmentSettings settings = new EnrollmentSettings(context);
                    String token = new TokenVault(context).load();
                    ApiClient client = new ApiClient(
                            settings.certificatePin(), settings.tlsName());
                    client.executeCallCommand(
                            settings.endpoint(), token, "dial", target);
                    monitor(client, settings.endpoint(), token);
                } catch (Exception error) {
                    fail();
                }
            }
        });
    }

    private void monitor(ApiClient client, String endpoint, String token) throws Exception {
        boolean progressed = false;
        int idlePolls = 0;
        while (!closed.get()) {
            String state = client.callState(endpoint, token);
            if ("active".equals(state)) {
                progressed = true;
                setActive();
                startAudioIfReady(client, endpoint, token);
            } else if ("dialing".equals(state)) {
                progressed = true;
                setDialing();
            } else if ("ended".equals(state) || "error".equals(state)
                    || (progressed && "idle".equals(state))) {
                remoteEnded();
                return;
            } else if (!progressed && "idle".equals(state) && ++idlePolls >= STARTUP_POLLS) {
                fail();
                return;
            }
            Thread.sleep(POLL_INTERVAL_MS);
        }
    }

    @Override
    public void onDisconnect() {
        finishLocally("hang_up", "");
    }

    @Override
    public void onAbort() {
        finishLocally("hang_up", "");
    }

    @Override
    public void onPlayDtmfTone(char tone) {
        if (closed.get()) {
            return;
        }
        commandExecutor.execute(new Runnable() {
            @Override public void run() {
                try {
                    EnrollmentSettings settings = new EnrollmentSettings(context);
                    new ApiClient(settings.certificatePin(), settings.tlsName())
                            .executeCallCommand(settings.endpoint(), new TokenVault(context).load(),
                                    "send_dtmf", Character.toString(tone));
                } catch (Exception ignored) {
                    // Telecom callbacks expose no private values or remote diagnostics.
                }
            }
        });
    }

    @Override
    public void onCallAudioStateChanged(CallAudioState state) {
        speakerphone = state != null
                && (state.getRoute() & CallAudioState.ROUTE_SPEAKER) != 0;
        AndroidCallAudioSession current = audioSession;
        if (current != null) {
            current.setSpeakerphone(speakerphone);
        }
    }

    private void startAudioIfReady(ApiClient client, String endpoint, String token) {
        if (audioSession != null || closed.get()) {
            return;
        }
        try {
            EnrollmentSettings settings = new EnrollmentSettings(context);
            MediaSessionCredentials credentials = client.createMediaSession(endpoint, token);
            AndroidCallAudioSession created = AndroidCallAudioSession.connect(context,
                    endpoint, settings.certificatePin(), settings.tlsName(), credentials);
            created.setSpeakerphone(speakerphone);
            created.start();
            if (closed.get()) {
                created.close();
            } else {
                audioSession = created;
            }
        } catch (Exception ignored) {
            // SCO may not be ready on the first active-state poll; retry without disclosure.
        }
    }

    private void finishLocally(String action, String value) {
        if (!closed.compareAndSet(false, true)) {
            return;
        }
        commandExecutor.execute(new Runnable() {
            @Override public void run() {
                try {
                    EnrollmentSettings settings = new EnrollmentSettings(context);
                    new ApiClient(settings.certificatePin(), settings.tlsName())
                            .executeCallCommand(settings.endpoint(), new TokenVault(context).load(),
                                    action, value);
                } catch (Exception ignored) {
                    // Local teardown continues even if the bridge is unavailable.
                } finally {
                    closeAudio();
                    setDisconnected(new DisconnectCause(DisconnectCause.LOCAL));
                    destroy();
                    monitorExecutor.shutdownNow();
                    commandExecutor.shutdown();
                }
            }
        });
    }

    private void remoteEnded() {
        if (closed.compareAndSet(false, true)) {
            closeAudio();
            setDisconnected(new DisconnectCause(DisconnectCause.REMOTE));
            destroy();
            monitorExecutor.shutdownNow();
            commandExecutor.shutdownNow();
        }
    }

    private void fail() {
        if (closed.compareAndSet(false, true)) {
            closeAudio();
            setDisconnected(new DisconnectCause(
                    DisconnectCause.ERROR, "AnalogBridge call failed"));
            destroy();
            monitorExecutor.shutdownNow();
            commandExecutor.shutdownNow();
        }
    }

    private void closeAudio() {
        AndroidCallAudioSession current = audioSession;
        audioSession = null;
        if (current != null) {
            current.close();
        }
    }
}
