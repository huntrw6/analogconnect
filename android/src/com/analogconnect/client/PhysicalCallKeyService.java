package com.analogconnect.client;

import android.accessibilityservice.AccessibilityService;
import android.content.Intent;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityEvent;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Filters only dedicated call keys and active-call DTMF; it never inspects UI content. */
public final class PhysicalCallKeyService extends AccessibilityService {
    static final String ACTION_DEMO_KEY = "com.analogconnect.client.DEMO_PHYSICAL_CALL_KEY";
    static final String EXTRA_KEY_CODE = "key_code";
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    @Override protected boolean onKeyEvent(KeyEvent event) {
        int keyCode = event.getKeyCode();
        String cachedState = PhysicalCallKeyState.current();
        boolean live = !("idle".equals(cachedState) || "ended".equals(cachedState)
                || "failed".equals(cachedState) || "error".equals(cachedState)
                || "connection_lost".equals(cachedState));
        boolean dedicated = keyCode == KeyEvent.KEYCODE_CALL
                || keyCode == KeyEvent.KEYCODE_ENDCALL
                || live && keyCode == KeyEvent.KEYCODE_POWER;
        boolean dtmf = "active".equals(cachedState) && keyCode >= KeyEvent.KEYCODE_0
                && keyCode <= KeyEvent.KEYCODE_POUND;
        if (!dedicated && !dtmf) return false;
        if (event.getAction() != KeyEvent.ACTION_DOWN || event.getRepeatCount() != 0) return true;

        if (new EnrollmentSettings(this).demoMode()) {
            sendBroadcast(new Intent(ACTION_DEMO_KEY).setPackage(getPackageName())
                    .putExtra(EXTRA_KEY_CODE, keyCode));
            return true;
        }
        final int capturedKey = keyCode;
        executor.execute(new Runnable() {
            @Override public void run() { dispatchProduction(capturedKey); }
        });
        return true;
    }

    private void dispatchProduction(int keyCode) {
        try {
            EnrollmentSettings settings = new EnrollmentSettings(this);
            ApiClient client = new ApiClient(settings.certificatePin(), settings.tlsName());
            String state = client.callState(settings.endpoint(), new TokenVault(this).load());
            PhysicalCallKeyDispatcher.Decision decision = PhysicalCallKeyDispatcher.dispatch(
                    state, keyCode, true, 0);
            String action = command(decision.action);
            if (action != null) client.executeCallCommand(settings.endpoint(),
                    new TokenVault(this).load(), action, decision.value);
            if (decision.action == PhysicalCallKeyDispatcher.Action.OPEN_DIALER) {
                startActivity(new Intent(this, CallsActivity.class)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            }
        } catch (Exception ignored) {
            // Fail closed: no command is sent when authoritative state cannot be read.
        }
    }

    private static String command(PhysicalCallKeyDispatcher.Action action) {
        if (action == PhysicalCallKeyDispatcher.Action.ANSWER) return "answer";
        if (action == PhysicalCallKeyDispatcher.Action.REJECT) return "reject";
        if (action == PhysicalCallKeyDispatcher.Action.HANG_UP) return "hang_up";
        if (action == PhysicalCallKeyDispatcher.Action.DTMF) return "send_dtmf";
        return null;
    }

    @Override public void onAccessibilityEvent(AccessibilityEvent event) { }
    @Override public void onInterrupt() { }

    @Override public void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }
}
