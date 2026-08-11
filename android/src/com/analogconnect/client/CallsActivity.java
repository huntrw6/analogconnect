package com.analogconnect.client;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.content.pm.PackageManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.IntentFilter;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.os.PowerManager;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.view.KeyEvent;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class CallsActivity extends Activity {
    static final String EXTRA_DIAL_TARGET = "com.analogconnect.client.DIAL_TARGET";
    static final String EXTRA_DEMO_CALL_STATE = "com.analogconnect.client.DEMO_CALL_STATE";
    static final String EXTRA_DISPLAY_NAME = "com.analogconnect.client.DISPLAY_NAME";
    private static final int RECORD_AUDIO_PERMISSION_REQUEST = 51;
    private static final long POLL_INTERVAL_MS = 500L;

    private final ExecutorService networkExecutor = Executors.newSingleThreadExecutor();
    private final ExecutorService audioExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final Object audioLock = new Object();
    private EnrollmentSettings settings;
    private TokenVault vault;
    private TextView title;
    private TextView duration;
    private TextView status;
    private TextView audioStatus;
    private TextView keypadLabel;
    private TextView callerName;
    private TextView physicalInstructions;
    private Button back;
    private EditText dialTarget;
    private Button dial;
    private Button answer;
    private Button reject;
    private Button hangUp;
    private LinearLayout keypad;
    private LinearLayout dialPad;
    private LinearLayout navigation;
    private Switch speakerphone;
    private boolean polling;
    private boolean commandPending;
    private volatile String callState = "idle";
    private long activeSinceElapsed;
    private AndroidCallAudioSession audioSession;
    private int audioGeneration;
    private boolean audioStarting;
    private boolean permissionRequested;
    private boolean permissionDenied;
    private boolean bridgeAvailable;
    private PowerManager.WakeLock proximityLock;
    private boolean keyReceiverRegistered;
    private final BroadcastReceiver demoKeyReceiver = new BroadcastReceiver() {
        @Override public void onReceive(Context context, android.content.Intent intent) {
            if (!settings.demoMode()) return;
            int keyCode = intent.getIntExtra(PhysicalCallKeyService.EXTRA_KEY_CODE, -1);
            handlePhysicalDecision(PhysicalCallKeyDispatcher.dispatch(callState, keyCode, true, 0));
        }
    };
    private long audioRetryAfterElapsed;

    private final Runnable poll = new Runnable() {
        @Override public void run() {
            if (!polling) {
                return;
            }
            networkExecutor.execute(new Runnable() {
                @Override public void run() {
                    String next = null;
                    try {
                        next = settings.demoMode() ? callState
                                : apiClient().callState(settings.endpoint(), vault.load());
                    } catch (Exception ignored) {
                        // The UI displays a fixed connectivity state without remote details.
                    }
                    final String result = next;
                    runOnUiThread(new Runnable() {
                        @Override public void run() {
                            if (!polling || isFinishing()) {
                                return;
                            }
                            if (result == null) {
                                status.setText("Bridge unavailable · retrying");
                                bridgeAvailable = false;
                                render(CallController.reduce(callState));
                            } else {
                                bridgeAvailable = true;
                                callState = result;
                                render(CallController.reduce(result));
                            }
                            mainHandler.postDelayed(poll, POLL_INTERVAL_MS);
                        }
                    });
                }
            });
        }
    };

    private final Runnable audioHealth = new Runnable() {
        @Override public void run() {
            AndroidCallAudioSession current;
            synchronized (audioLock) {
                current = audioSession;
            }
            if (current == null) {
                return;
            }
            String error = current.errorCode();
            if (error == null) {
                audioStatus.setText("Call audio connected");
                mainHandler.postDelayed(this, POLL_INTERVAL_MS);
                return;
            }
            synchronized (audioLock) {
                if (audioSession != current) {
                    return;
                }
                audioSession = null;
                audioGeneration++;
            }
            audioStatus.setText("Call audio interrupted · retrying");
            audioExecutor.execute(new Runnable() {
                @Override public void run() { current.close(); }
            });
            if ("active".equals(callState)) {
                mainHandler.postDelayed(new Runnable() {
                    @Override public void run() { startAudioIfAllowed(); }
                }, 1000L);
            }
        }
    };

    @Override protected void onCreate(Bundle state) {
        Ui.applyTheme(this);
        super.onCreate(state);
        settings = new EnrollmentSettings(this);
        vault = new TokenVault(this);
        PowerManager power = (PowerManager) getSystemService(POWER_SERVICE);
        if (power != null && power.isWakeLockLevelSupported(
                PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK)) {
            proximityLock = power.newWakeLock(PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK,
                    "AnalogConnect:call-proximity");
            proximityLock.setReferenceCounted(false);
        }
        if (state != null) {
            callState = state.getString("call_state", callState);
        } else if (settings.demoMode()) {
            String demoState = getIntent().getStringExtra(EXTRA_DEMO_CALL_STATE);
            if (demoState != null) callState = demoState;
        }

        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setFocusableInTouchMode(true);
        int padding = dp(12);
        content.setPadding(padding, padding, padding, padding);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(content);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 1f));

        back = new Button(this);
        back.setText("Back");
        back.setContentDescription("Back to AnalogConnect home");
        back.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { finish(); }
        });
        back.setVisibility(View.GONE);

        title = text("Calls", 30);
        title.setGravity(Gravity.CENTER_HORIZONTAL);
        title.setPadding(0, dp(8), 0, dp(4));
        content.addView(title);

        String displayName = getIntent().getStringExtra(EXTRA_DISPLAY_NAME);
        callerName = text(displayName == null ? "" : displayName, 22);
        callerName.setGravity(Gravity.CENTER_HORIZONTAL);
        callerName.setTypeface(null, android.graphics.Typeface.BOLD);
        callerName.setPadding(0, 0, 0, dp(8));
        callerName.setVisibility(displayName == null || displayName.isEmpty()
                ? View.GONE : View.VISIBLE);
        content.addView(callerName);

        duration = text("", 22);
        duration.setGravity(Gravity.CENTER_HORIZONTAL);
        content.addView(duration);

        status = text("Connecting to bridge…", 16);
        status.setGravity(Gravity.CENTER_HORIZONTAL);
        status.setPadding(0, dp(4), 0, dp(8));
        status.setContentDescription("Call connection status");
        content.addView(status);

        physicalInstructions = text("", 17);
        physicalInstructions.setGravity(Gravity.CENTER_HORIZONTAL);
        physicalInstructions.setPadding(dp(8), dp(8), dp(8), dp(12));
        physicalInstructions.setContentDescription("Physical call button instructions");
        content.addView(physicalInstructions);

        dialTarget = new EditText(this);
        dialTarget.setId(R.id.dial_target);
        dialTarget.setHint("Phone number");
        dialTarget.setSingleLine(true);
        dialTarget.setInputType(InputType.TYPE_CLASS_PHONE);
        dialTarget.setContentDescription("Phone number to call");
        String initialTarget = getIntent().getStringExtra(EXTRA_DIAL_TARGET);
        if (initialTarget != null && initialTarget.length() <= 128) {
            dialTarget.setText(initialTarget);
        }
        content.addView(dialTarget);

        keypadLabel = text("Keypad", 18);
        keypadLabel.setGravity(Gravity.CENTER_HORIZONTAL);
        keypadLabel.setPadding(0, dp(4), 0, dp(2));
        content.addView(keypadLabel);
        dialPad = new LinearLayout(this);
        dialPad.setOrientation(LinearLayout.VERTICAL);
        String[][] dialKeys = new String[][] {
                {"1", "2\nABC", "3\nDEF"}, {"4\nGHI", "5\nJKL", "6\nMNO"},
                {"7\nPQRS", "8\nTUV", "9\nWXYZ"}, {"*", "0\n+", "#"}};
        for (final String[] row : dialKeys) {
            LinearLayout keys = new LinearLayout(this);
            keys.setOrientation(LinearLayout.HORIZONTAL);
            for (final String label : row) {
                Button key = button(label, new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        dialTarget.append(label.substring(0, 1));
                    }
                });
                key.setContentDescription("Dial " + label.substring(0, 1));
                keys.addView(key, new LinearLayout.LayoutParams(0, dp(44), 1f));
            }
            dialPad.addView(keys);
        }
        Button delete = button("Delete", new View.OnClickListener() {
            @Override public void onClick(View view) {
                int length = dialTarget.length();
                if (length > 0) dialTarget.getText().delete(length - 1, length);
            }
        });
        delete.setContentDescription("Delete last digit");
        dialPad.addView(delete);
        content.addView(dialPad);

        dial = button("Review and call", new View.OnClickListener() {
            @Override public void onClick(View view) { confirmDial(); }
        });
        content.addView(dial);

        answer = button("Answer", new View.OnClickListener() {
            @Override public void onClick(View view) { execute("answer", ""); }
        });
        answer.setTextColor(android.graphics.Color.WHITE);
        answer.setBackgroundColor(Ui.GREEN);
        content.addView(answer);

        reject = button("Reject", new View.OnClickListener() {
            @Override public void onClick(View view) { execute("reject", ""); }
        });
        reject.setTextColor(android.graphics.Color.WHITE);
        reject.setBackgroundColor(Ui.RED);
        content.addView(reject);

        hangUp = button("End call", new View.OnClickListener() {
            @Override public void onClick(View view) { confirmHangUp(); }
        });
        content.addView(hangUp);

        speakerphone = new Switch(this);
        speakerphone.setText("Speakerphone");
        speakerphone.setContentDescription("Route call audio through speakerphone");
        speakerphone.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
            @Override public void onCheckedChanged(CompoundButton button, boolean checked) {
                setSpeakerphone(checked);
            }
        });
        content.addView(speakerphone);

        audioStatus = text("Call audio starts automatically when connected", 15);
        audioStatus.setPadding(0, dp(6), 0, dp(16));
        content.addView(audioStatus);

        keypad = new LinearLayout(this);
        keypad.setOrientation(LinearLayout.VERTICAL);
        for (String row : new String[] {"123", "456", "789", "*0#"}) {
            LinearLayout keys = new LinearLayout(this);
            keys.setOrientation(LinearLayout.HORIZONTAL);
            for (int index = 0; index < row.length(); index++) {
                final String tone = Character.toString(row.charAt(index));
                Button key = button(tone, new View.OnClickListener() {
                    @Override public void onClick(View view) { sendTone(tone); }
                });
                key.setContentDescription("Send DTMF " + tone);
                keys.addView(key, new LinearLayout.LayoutParams(0, dp(56), 1.0f));
            }
            keypad.addView(keys);
        }
        content.addView(keypad);
        navigation = Ui.bottomNavigation(this, "Calls");
        root.addView(navigation);
        setContentView(root);
        render(CallController.reduce("idle"));
        content.requestFocus();
    }

    private TextView text(String value, int size) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(size);
        return view;
    }

    private Button button(String label, View.OnClickListener listener) {
        Button button = new Button(this);
        button.setText(label);
        button.setOnClickListener(listener);
        return button;
    }

    private void render(CallController.State state) {
        PhysicalCallKeyState.update(state.call);
        title.setText(state.title);
        boolean enabled = !commandPending && (settings.demoMode() || bridgeAvailable);
        dialTarget.setVisibility(state.canDial ? View.VISIBLE : View.GONE);
        dialPad.setVisibility(state.canDial ? View.VISIBLE : View.GONE);
        keypadLabel.setVisibility(state.canDial ? View.VISIBLE : View.GONE);
        dial.setVisibility(state.canDial ? View.VISIBLE : View.GONE);
        dial.setEnabled(enabled && state.canDial);
        answer.setVisibility(View.GONE);
        answer.setClickable(false);
        reject.setVisibility(View.GONE);
        reject.setClickable(false);
        hangUp.setVisibility(View.GONE);
        hangUp.setClickable(false);
        keypad.setVisibility(View.GONE);
        speakerphone.setVisibility(View.GONE);
        speakerphone.setClickable(false);
        navigation.setVisibility(state.canDial ? View.VISIBLE : View.GONE);
        physicalInstructions.setText(instructions(state.call));
        physicalInstructions.setVisibility(state.canDial ? View.GONE : View.VISIBLE);
        audioStatus.setVisibility(state.needsAudio ? View.VISIBLE : View.GONE);
        updateProximity(!state.canDial && !"ended".equals(state.call)
                && !"failed".equals(state.call) && !"error".equals(state.call));
        if (state.needsAudio) {
            if (activeSinceElapsed == 0L) {
                activeSinceElapsed = SystemClock.elapsedRealtime();
            }
            long elapsed = Math.max(0L, SystemClock.elapsedRealtime() - activeSinceElapsed);
            duration.setText(String.format("%02d:%02d", elapsed / 60000L,
                    (elapsed / 1000L) % 60L));
            duration.setVisibility(View.VISIBLE);
            if (settings.demoMode()) {
                audioStatus.setText("Offline demo · call audio is simulated");
            } else {
                startAudioIfAllowed();
            }
        } else {
            activeSinceElapsed = 0L;
            duration.setText("");
            duration.setVisibility(View.GONE);
            if (audioSession != null) {
                stopAudio();
            }
        }
        if (!commandPending) {
            status.setText(settings.demoMode() ? "Offline demo · no real call actions"
                    : bridgeAvailable ? "iPhone calls connected" : "Calls unavailable · retrying");
        }
    }

    private void confirmDial() {
        final String target = dialTarget.getText().toString().trim();
        if (target.isEmpty()) {
            dialTarget.setError("Phone number is required");
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle("Place this call?")
                .setMessage(target)
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Call", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        execute("dial", target);
                    }
                })
                .show();
    }

    private void confirmHangUp() {
        new AlertDialog.Builder(this)
                .setTitle("End this call?")
                .setNegativeButton("Cancel", null)
                .setPositiveButton("End call", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        execute("hang_up", "");
                    }
                })
                .show();
    }

    private void sendTone(String tone) {
        if (CallController.validDtmf(tone) && "active".equals(callState)) {
            execute("send_dtmf", tone);
        }
    }

    private void execute(final String action, final String value) {
        if (commandPending) {
            return;
        }
        commandPending = true;
        if (settings.demoMode()) {
            if ("dial".equals(action)) callState = "dialing";
            else if ("answer".equals(action)) callState = "active";
            else if ("reject".equals(action) || "hang_up".equals(action)) callState = "ended";
            commandPending = false;
            render(CallController.reduce(callState));
            status.setText("Offline demo · no real call action was sent");
            return;
        }
        render(CallController.reduce(callState));
        status.setText("Sending call command…");
        networkExecutor.execute(new Runnable() {
            @Override public void run() {
                boolean accepted = false;
                try {
                    apiClient().executeCallCommand(
                            settings.endpoint(), vault.load(), action, value);
                    accepted = true;
                } catch (Exception ignored) {
                    // Fixed UI failure avoids disclosing command values or backend details.
                }
                final boolean finalAccepted = accepted;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        commandPending = false;
                        render(CallController.reduce(callState));
                        if (finalAccepted) {
                            status.setText("Command accepted");
                        } else {
                            status.setText("Call command failed · try again");
                        }
                    }
                });
            }
        });
    }

    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        int keyCode = event.getKeyCode();
        boolean relevant = PhysicalCallKeyDispatcher.isCallKey(keyCode);
        PhysicalCallKeyDispatcher.Decision decision = PhysicalCallKeyDispatcher.dispatch(
                callState, keyCode, event.getAction() == KeyEvent.ACTION_DOWN,
                event.getRepeatCount());
        if (decision.action != PhysicalCallKeyDispatcher.Action.NONE) {
            handlePhysicalDecision(decision);
            return true;
        }
        boolean live = !("idle".equals(callState) || "ended".equals(callState)
                || "failed".equals(callState) || "error".equals(callState)
                || "connection_lost".equals(callState));
        if ((keyCode == KeyEvent.KEYCODE_CALL || keyCode == KeyEvent.KEYCODE_ENDCALL)
                || live && relevant) return true;
        return super.dispatchKeyEvent(event);
    }

    private void handlePhysicalDecision(PhysicalCallKeyDispatcher.Decision decision) {
        if (decision.action == PhysicalCallKeyDispatcher.Action.OPEN_DIALER) {
            dialTarget.requestFocus();
            return;
        }
        if (decision.action == PhysicalCallKeyDispatcher.Action.ANSWER) {
            execute("answer", "");
            return;
        }
        if (decision.action == PhysicalCallKeyDispatcher.Action.REJECT) {
            execute("reject", "");
            return;
        }
        if (decision.action == PhysicalCallKeyDispatcher.Action.HANG_UP) {
            execute("hang_up", "");
            return;
        }
        if (decision.action == PhysicalCallKeyDispatcher.Action.DTMF) {
            execute("send_dtmf", decision.value);
            status.setText("Tone " + decision.value + " sent");
            return;
        }
    }

    private static String instructions(String state) {
        if ("incoming".equals(state)) {
            return "Press the green Call key to answer\nPress the red End key to decline";
        }
        if ("active".equals(state)) {
            return "Press the red End key to end the call\n"
                    + "Use the physical number keys for touch tones";
        }
        if ("dialing".equals(state) || "ringing".equals(state)) {
            return "Press the red End key to cancel the call";
        }
        if ("ending".equals(state)) return "Ending call…";
        return "";
    }

    private void updateProximity(boolean live) {
        if (proximityLock == null) return;
        if (live && !proximityLock.isHeld()) proximityLock.acquire();
        if (!live && proximityLock.isHeld()) proximityLock.release();
    }

    private ApiClient apiClient() throws Exception {
        return new ApiClient(settings.certificatePin(), settings.tlsName());
    }

    private void startAudioIfAllowed() {
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO)
                != PackageManager.PERMISSION_GRANTED) {
            audioStatus.setText("Microphone permission is needed for call audio");
            if (!permissionRequested && !permissionDenied) {
                permissionRequested = true;
                requestPermissions(new String[] {Manifest.permission.RECORD_AUDIO},
                        RECORD_AUDIO_PERMISSION_REQUEST);
            }
            return;
        }
        startAudio();
    }

    private void startAudio() {
        final int generation;
        synchronized (audioLock) {
            if (audioSession != null || audioStarting
                    || SystemClock.elapsedRealtime() < audioRetryAfterElapsed) {
                return;
            }
            audioStarting = true;
            generation = ++audioGeneration;
        }
        audioStatus.setText("Connecting call audio…");
        final boolean useSpeaker = speakerphone.isChecked();
        audioExecutor.execute(new Runnable() {
            @Override public void run() {
                AndroidCallAudioSession created = null;
                boolean active = false;
                try {
                    ApiClient client = apiClient();
                    MediaSessionCredentials credentials = client.createMediaSession(
                            settings.endpoint(), vault.load());
                    created = AndroidCallAudioSession.connect(getApplicationContext(),
                            settings.endpoint(), settings.certificatePin(), settings.tlsName(),
                            credentials);
                    created.setSpeakerphone(useSpeaker);
                    created.start();
                    synchronized (audioLock) {
                        if (generation == audioGeneration && audioSession == null
                                && "active".equals(callState)) {
                            audioSession = created;
                            created = null;
                            active = true;
                        }
                        if (generation == audioGeneration) {
                            audioStarting = false;
                            if (!active) {
                                audioRetryAfterElapsed = SystemClock.elapsedRealtime() + 2000L;
                            }
                        }
                    }
                } catch (Exception ignored) {
                    // Fixed status below; media credentials and transport details stay private.
                } finally {
                    if (created != null) {
                        created.close();
                    }
                }
                final boolean connected = active;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        if (generation == audioGeneration && !isFinishing()) {
                            audioStatus.setText(connected
                                    ? "Call audio connected" : "Call audio unavailable · retrying");
                            if (connected) {
                                mainHandler.removeCallbacks(audioHealth);
                                mainHandler.postDelayed(audioHealth, POLL_INTERVAL_MS);
                            }
                        }
                    }
                });
            }
        });
    }

    private void setSpeakerphone(final boolean enabled) {
        final AndroidCallAudioSession current;
        synchronized (audioLock) {
            current = audioSession;
        }
        if (current == null) {
            return;
        }
        audioExecutor.execute(new Runnable() {
            @Override public void run() {
                try {
                    current.setSpeakerphone(enabled);
                } catch (RuntimeException ignored) {
                    runOnUiThread(new Runnable() {
                        @Override public void run() {
                            audioStatus.setText("Audio route could not be changed");
                        }
                    });
                }
            }
        });
    }

    private void stopAudio() {
        mainHandler.removeCallbacks(audioHealth);
        final AndroidCallAudioSession current;
        synchronized (audioLock) {
            audioGeneration++;
            audioStarting = false;
            audioRetryAfterElapsed = 0L;
            current = audioSession;
            audioSession = null;
        }
        audioStatus.setText("Call audio stopped");
        audioExecutor.execute(new Runnable() {
            @Override public void run() {
                if (current != null) {
                    current.close();
                }
            }
        });
    }

    @Override public void onRequestPermissionsResult(int requestCode, String[] permissions,
            int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode == RECORD_AUDIO_PERMISSION_REQUEST) {
            permissionRequested = false;
        }
        if (requestCode == RECORD_AUDIO_PERMISSION_REQUEST && grantResults.length == 1
                && grantResults[0] == PackageManager.PERMISSION_GRANTED
                && "active".equals(callState)) {
            startAudio();
        } else if (requestCode == RECORD_AUDIO_PERMISSION_REQUEST) {
            permissionDenied = true;
            audioStatus.setText("Call audio disabled without microphone permission");
        }
    }

    @Override protected void onStart() {
        super.onStart();
        registerReceiver(demoKeyReceiver, new IntentFilter(PhysicalCallKeyService.ACTION_DEMO_KEY));
        keyReceiverRegistered = true;
        polling = true;
        mainHandler.removeCallbacks(poll);
        mainHandler.post(poll);
    }

    @Override protected void onStop() {
        polling = false;
        mainHandler.removeCallbacks(poll);
        if (keyReceiverRegistered) {
            unregisterReceiver(demoKeyReceiver);
            keyReceiverRegistered = false;
        }
        super.onStop();
    }

    @Override protected void onDestroy() {
        polling = false;
        mainHandler.removeCallbacksAndMessages(null);
        stopAudio();
        networkExecutor.shutdownNow();
        audioExecutor.shutdown();
        updateProximity(false);
        super.onDestroy();
    }

    @Override public void onBackPressed() {
        if ("idle".equals(callState) || "ended".equals(callState)
                || "error".equals(callState) || "failed".equals(callState)
                || "connection_lost".equals(callState)) {
            super.onBackPressed();
        } else {
            status.setText("Use the visible call controls before leaving");
        }
    }

    @Override protected void onSaveInstanceState(Bundle state) {
        super.onSaveInstanceState(state);
        state.putString("call_state", callState);
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
