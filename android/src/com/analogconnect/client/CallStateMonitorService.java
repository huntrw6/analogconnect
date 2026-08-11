package com.analogconnect.client;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.IBinder;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

/** Keeps backend-authoritative incoming calls visible while another screen is active. */
public final class CallStateMonitorService extends Service {
    private static final String CHANNEL = "analog_connection";
    private static final int NOTIFICATION_ID = 9100;
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private final AtomicBoolean running = new AtomicBoolean();
    private final MessageNotificationTracker messageTracker = new MessageNotificationTracker();

    static void start(Context context) {
        context.startForegroundService(new Intent(context, CallStateMonitorService.class));
    }

    @Override public void onCreate() {
        super.onCreate();
        createChannel();
        startForeground(NOTIFICATION_ID, statusNotification());
    }

    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (running.compareAndSet(false, true)) {
            executor.execute(new Runnable() {
                @Override public void run() { monitor(); }
            });
        }
        return START_STICKY;
    }

    private void monitor() {
        String previous = "";
        long delay = 750L;
        long nextMessagePoll = 0L;
        while (running.get()) {
            try {
                EnrollmentSettings settings = new EnrollmentSettings(this);
                if (settings.demoMode()) {
                    previous = "";
                } else {
                    ApiClient client = new ApiClient(
                            settings.certificatePin(), settings.tlsName());
                    String state = client.callState(settings.endpoint(), new TokenVault(this).load());
                    if (CallMonitorTransition.shouldShowIncoming(previous, state)) {
                        AnalogNotifications.showIncomingCall(this, "Incoming call");
                        AnalogNotifications.openIncomingCallScreen(this);
                    } else if (CallMonitorTransition.shouldCancelIncoming(previous, state)) {
                        AnalogNotifications.cancelIncomingCall(this);
                    }
                    previous = state;
                    long now = android.os.SystemClock.elapsedRealtime();
                    if (now >= nextMessagePoll) {
                        pollMessages(client, settings);
                        nextMessagePoll = now + 5_000L;
                    }
                }
                delay = 750L;
            } catch (Exception ignored) {
                delay = Math.min(10_000L, delay * 2L);
            }
            try {
                Thread.sleep(delay);
            } catch (InterruptedException interrupted) {
                Thread.currentThread().interrupt();
                return;
            }
        }
    }

    private void pollMessages(ApiClient client, EnrollmentSettings settings) throws Exception {
        ConversationPageData<ConversationSummary> page = client.conversations(
                settings.endpoint(), new TokenVault(this).load());
        new OfflineCache(this).storeConversations(page);
        for (ConversationSummary item : messageTracker.update(page.items)) {
            AnalogNotifications.showMessage(this,
                    MessageNotificationTracker.notificationId(item.id), item.id,
                    item.displayLabel(), item.previewLabel());
        }
    }

    private void createChannel() {
        NotificationChannel channel = new NotificationChannel(CHANNEL, "Connection",
                NotificationManager.IMPORTANCE_LOW);
        channel.setDescription("Keeps iPhone call detection available");
        ((NotificationManager) getSystemService(NOTIFICATION_SERVICE))
                .createNotificationChannel(channel);
    }

    private Notification statusNotification() {
        return new Notification.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
                .setContentTitle("AnalogConnect is ready")
                .setContentText("Watching for iPhone calls")
                .setOngoing(true).build();
    }

    @Override public void onDestroy() {
        running.set(false);
        executor.shutdownNow();
        super.onDestroy();
    }

    @Override public IBinder onBind(Intent intent) { return null; }
}
