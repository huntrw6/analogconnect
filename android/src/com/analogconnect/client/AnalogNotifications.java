package com.analogconnect.client;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;

final class AnalogNotifications {
    private static final String MESSAGES = "analog_messages";
    private static final String CALLS = "analog_calls";

    private AnalogNotifications() {}

    static void createChannels(Context context) {
        NotificationManager manager = (NotificationManager)
                context.getSystemService(Context.NOTIFICATION_SERVICE);
        NotificationChannel messages = new NotificationChannel(MESSAGES, "Messages",
                NotificationManager.IMPORTANCE_HIGH);
        messages.setDescription("New iPhone messages");
        NotificationChannel calls = new NotificationChannel(CALLS, "Incoming calls",
                NotificationManager.IMPORTANCE_HIGH);
        calls.setDescription("Calls arriving through your iPhone");
        calls.enableVibration(true);
        calls.setLightColor(Color.GREEN);
        manager.createNotificationChannel(messages);
        manager.createNotificationChannel(calls);
    }

    static void showMessage(Context context, int id, String conversationId, String title,
            String preview) {
        createChannels(context);
        Intent open = new Intent(context, ConversationsActivity.class)
                .putExtra(ConversationsActivity.EXTRA_CONVERSATION_ID, conversationId);
        PendingIntent pending = PendingIntent.getActivity(context, id, open,
                PendingIntent.FLAG_UPDATE_CURRENT);
        Notification notification = new Notification.Builder(context, MESSAGES)
                .setSmallIcon(android.R.drawable.sym_action_chat)
                .setContentTitle(title).setContentText(preview).setStyle(
                        new Notification.BigTextStyle().bigText(preview))
                .setContentIntent(pending).setAutoCancel(true).setShowWhen(true).build();
        ((NotificationManager) context.getSystemService(Context.NOTIFICATION_SERVICE))
                .notify(id, notification);
    }

    static void showIncomingCall(Context context, String displayName) {
        createChannels(context);
        Intent open = new Intent(context, CallsActivity.class)
                .putExtra(CallsActivity.EXTRA_DEMO_CALL_STATE, "incoming")
                .putExtra(CallsActivity.EXTRA_DISPLAY_NAME, displayName);
        PendingIntent pending = PendingIntent.getActivity(context, 9001, open,
                PendingIntent.FLAG_UPDATE_CURRENT);
        Notification notification = new Notification.Builder(context, CALLS)
                .setSmallIcon(android.R.drawable.sym_call_incoming)
                .setContentTitle(displayName).setContentText("Incoming call")
                .setCategory(Notification.CATEGORY_CALL).setOngoing(true)
                .setPriority(Notification.PRIORITY_HIGH).setFullScreenIntent(pending, true)
                .setContentIntent(pending).build();
        ((NotificationManager) context.getSystemService(Context.NOTIFICATION_SERVICE))
                .notify(9001, notification);
    }
}
