package com.analogconnect.client;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.util.Log;

/** Receives privacy-safe physical key codes from the Pi's permission-gated ADB monitor. */
public final class PhysicalCallKeyReceiver extends BroadcastReceiver {
    static final String ACTION_KEY = "com.analogconnect.client.PHYSICAL_CALL_KEY";
    static final String EXTRA_KEY_CODE = "key_code";

    @Override public void onReceive(Context context, Intent intent) {
        if (!ACTION_KEY.equals(intent.getAction())) return;
        final int keyCode = intent.getIntExtra(EXTRA_KEY_CODE, -1);
        if (!PhysicalCallKeyDispatcher.isCallKey(keyCode)) return;
        final PendingResult pending = goAsync();
        final Context app = context.getApplicationContext();
        new Thread(new Runnable() {
            @Override public void run() {
                try {
                    EnrollmentSettings settings = new EnrollmentSettings(app);
                    if (settings.demoMode()) {
                        Log.i("AnalogCallKeys", "ADB physical key received key=" + keyCode);
                        showCallScreen(app);
                        Thread.sleep(800L);
                        app.sendBroadcast(new Intent(PhysicalCallKeyService.ACTION_DEMO_KEY)
                                .setPackage(app.getPackageName())
                                .putExtra(PhysicalCallKeyService.EXTRA_KEY_CODE, keyCode));
                        return;
                    }
                    ApiClient client = new ApiClient(
                            settings.certificatePin(), settings.tlsName());
                    String token = new TokenVault(app).load();
                    String state = client.callState(settings.endpoint(), token);
                    PhysicalCallKeyDispatcher.Decision decision =
                            PhysicalCallKeyDispatcher.dispatch(state, keyCode, true, 0);
                    String command = command(decision.action);
                    if (command != null) client.executeCallCommand(
                            settings.endpoint(), token, command, decision.value);
                    if (decision.action != PhysicalCallKeyDispatcher.Action.NONE) {
                        showCallScreen(app);
                    }
                } catch (Exception ignored) {
                    // Fail closed when authoritative state or credentials are unavailable.
                } finally {
                    pending.finish();
                }
            }
        }, "physical-call-key").start();
    }

    private static void showCallScreen(Context context) {
        context.startActivity(new Intent(context, CallsActivity.class)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                        | Intent.FLAG_ACTIVITY_REORDER_TO_FRONT
                        | Intent.FLAG_ACTIVITY_SINGLE_TOP));
    }

    private static String command(PhysicalCallKeyDispatcher.Action action) {
        if (action == PhysicalCallKeyDispatcher.Action.ANSWER) return "answer";
        if (action == PhysicalCallKeyDispatcher.Action.REJECT) return "reject";
        if (action == PhysicalCallKeyDispatcher.Action.HANG_UP) return "hang_up";
        if (action == PhysicalCallKeyDispatcher.Action.DTMF) return "send_dtmf";
        return null;
    }
}
