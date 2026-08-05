package com.analogconnect.client;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.text.method.HideReturnsTransformationMethod;
import android.text.method.PasswordTransformationMethod;
import android.view.View;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
    private static final int RECORD_AUDIO_PERMISSION_REQUEST = 41;

    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private final ExecutorService audioExecutor = Executors.newSingleThreadExecutor();
    private final Object audioLock = new Object();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private EditText endpoint;
    private EditText token;
    private EditText certificatePin;
    private EditText tlsName;
    private EditText recipient;
    private EditText messageBody;
    private EditText dialTarget;
    private EditText dtmfTone;
    private TextView result;
    private TokenVault vault;
    private NsdDiscovery discovery;
    private EnrollmentSettings settings;
    private Button startAudio;
    private Button stopAudio;
    private Switch speakerphone;
    private AnalogPhoneIntegration phoneIntegration;
    private boolean updatingPhoneIntegration;
    private AndroidCallAudioSession audioSession;
    private int audioGeneration;
    private final Runnable audioHealthCheck = new Runnable() {
        @Override public void run() {
            AndroidCallAudioSession current;
            synchronized (audioLock) {
                current = audioSession;
            }
            if (current == null) {
                return;
            }
            String errorCode = current.errorCode();
            if (errorCode == null) {
                AudioJitterBuffer.Summary audio = current.jitterSummary();
                long pacingPartsPerMillion = current.pacingAdjustmentNanos() * 1_000_000L
                        / 7_500_000L;
                result.setText("Call audio active · buffer " + audio.depth
                        + " · holds " + current.concealedFrames()
                        + " · late " + audio.late
                        + " · overflow " + audio.overflow
                        + " · trims " + current.trimmedFrames()
                        + " · pace " + (pacingPartsPerMillion >= 0 ? "+" : "")
                        + pacingPartsPerMillion + " ppm");
                mainHandler.postDelayed(this, 500);
                return;
            }
            synchronized (audioLock) {
                if (audioSession != current) {
                    return;
                }
                audioGeneration++;
                audioSession = null;
            }
            startAudio.setEnabled(true);
            stopAudio.setEnabled(false);
            result.setText("Call audio stopped: " + errorCode);
            audioExecutor.execute(new Runnable() {
                @Override public void run() { current.close(); }
            });
        }
    };

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        vault = new TokenVault(this);
        discovery = new NsdDiscovery(this);
        settings = new EnrollmentSettings(this);
        settings.migrateLegacy(getPreferences(MODE_PRIVATE));
        phoneIntegration = new AnalogPhoneIntegration(this);

        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(24);
        layout.setPadding(padding, padding, padding, padding);

        ScrollView scroll = new ScrollView(this);
        scroll.addView(layout);

        TextView title = new TextView(this);
        title.setText("AnalogConnect");
        title.setTextSize(28);
        layout.addView(title);

        TextView subtitle = new TextView(this);
        subtitle.setText("Raspberry Pi companion client\nAndroid 8.1 hardware check");
        subtitle.setPadding(0, dp(8), 0, dp(20));
        layout.addView(subtitle);

        endpoint = new EditText(this);
        endpoint.setId(R.id.endpoint);
        endpoint.setHint("Daemon endpoint");
        endpoint.setSingleLine(true);
        endpoint.setText(settings.endpoint());
        layout.addView(endpoint);

        Button discover = new Button(this);
        discover.setId(R.id.discover_daemon);
        discover.setText("Discover daemon");
        discover.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { discoverDaemon(); }
        });
        layout.addView(discover);

        tlsName = new EditText(this);
        tlsName.setId(R.id.tls_name);
        tlsName.setHint("TLS server name");
        tlsName.setSingleLine(true);
        tlsName.setText(settings.tlsName());
        layout.addView(tlsName);

        certificatePin = new EditText(this);
        certificatePin.setId(R.id.certificate_pin);
        certificatePin.setHint("HTTPS certificate SHA-256 pin");
        certificatePin.setSingleLine(true);
        certificatePin.setText(settings.certificatePin());
        layout.addView(certificatePin);

        token = new EditText(this);
        token.setId(R.id.enrollment_token);
        token.setHint("Enrollment token");
        token.setSingleLine(true);
        token.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        layout.addView(token);

        Switch showToken = new Switch(this);
        showToken.setText("Show token");
        showToken.setChecked(false);
        showToken.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
            @Override
            public void onCheckedChanged(CompoundButton button, boolean checked) {
                token.setTransformationMethod(checked
                        ? HideReturnsTransformationMethod.getInstance()
                        : PasswordTransformationMethod.getInstance());
                token.setSelection(token.length());
            }
        });
        layout.addView(showToken);

        Button save = new Button(this);
        save.setId(R.id.save_enrollment);
        save.setText("Save enrollment");
        save.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                saveEnrollment();
            }
        });
        layout.addView(save);

        Button clearEnrollment = new Button(this);
        clearEnrollment.setId(R.id.clear_enrollment);
        clearEnrollment.setText("Clear enrollment token");
        clearEnrollment.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { confirmClearEnrollment(); }
        });
        layout.addView(clearEnrollment);

        Button check = new Button(this);
        check.setId(R.id.check_daemon);
        check.setText("Check daemon");
        check.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                checkDaemon();
            }
        });
        layout.addView(check);

        TextView messageTitle = new TextView(this);
        messageTitle.setText("Send SMS through iPhone");
        messageTitle.setTextSize(20);
        messageTitle.setPadding(0, dp(24), 0, dp(8));
        layout.addView(messageTitle);

        recipient = new EditText(this);
        recipient.setHint("Recipient number");
        recipient.setSingleLine(true);
        recipient.setInputType(InputType.TYPE_CLASS_PHONE);
        layout.addView(recipient);

        messageBody = new EditText(this);
        messageBody.setHint("Message");
        messageBody.setMaxLines(5);
        messageBody.setInputType(InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
                | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        layout.addView(messageBody);

        Button send = new Button(this);
        send.setText("Review and send");
        send.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                confirmSend();
            }
        });
        layout.addView(send);

        TextView callTitle = new TextView(this);
        callTitle.setText("Call controls");
        callTitle.setTextSize(20);
        callTitle.setPadding(0, dp(24), 0, dp(8));
        layout.addView(callTitle);

        dialTarget = new EditText(this);
        dialTarget.setHint("Number to dial");
        dialTarget.setSingleLine(true);
        dialTarget.setInputType(InputType.TYPE_CLASS_PHONE);
        layout.addView(dialTarget);

        Button dial = new Button(this);
        dial.setText("Review and dial");
        dial.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                String target = dialTarget.getText().toString().trim();
                if (target.isEmpty()) {
                    result.setText("Dial target is required");
                } else {
                    confirmCallCommand("Place this call?", "Dial: " + target, "dial", target);
                }
            }
        });
        layout.addView(dial);

        Button answer = new Button(this);
        answer.setText("Answer incoming call");
        answer.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                executeCallCommand("answer", "");
            }
        });
        layout.addView(answer);

        Button hangUp = new Button(this);
        hangUp.setText("Hang up or reject");
        hangUp.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                confirmCallCommand("End the call?", "This will hang up or reject the current call.",
                        "hang_up", "");
            }
        });
        layout.addView(hangUp);

        dtmfTone = new EditText(this);
        dtmfTone.setHint("DTMF tone");
        dtmfTone.setSingleLine(true);
        dtmfTone.setInputType(InputType.TYPE_CLASS_PHONE);
        layout.addView(dtmfTone);

        Button sendTone = new Button(this);
        sendTone.setText("Send DTMF tone");
        sendTone.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                String tone = dtmfTone.getText().toString().trim();
                if (tone.length() != 1) {
                    result.setText("Enter exactly one DTMF tone");
                } else {
                    executeCallCommand("send_dtmf", tone);
                }
            }
        });
        layout.addView(sendTone);

        startAudio = new Button(this);
        startAudio.setId(R.id.start_call_audio);
        startAudio.setText("Start call audio");
        startAudio.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { requestStartCallAudio(); }
        });
        layout.addView(startAudio);

        stopAudio = new Button(this);
        stopAudio.setId(R.id.stop_call_audio);
        stopAudio.setText("Stop call audio");
        stopAudio.setEnabled(false);
        stopAudio.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { stopCallAudio(true); }
        });
        layout.addView(stopAudio);

        speakerphone = new Switch(this);
        speakerphone.setId(R.id.call_audio_speakerphone);
        speakerphone.setText("Speakerphone");
        speakerphone.setChecked(false);
        speakerphone.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
            @Override public void onCheckedChanged(CompoundButton button, boolean checked) {
                setCallAudioSpeakerphone(checked);
            }
        });
        layout.addView(speakerphone);

        TextView phoneIntegrationTitle = new TextView(this);
        phoneIntegrationTitle.setText("Android Phone integration (experimental)");
        phoneIntegrationTitle.setTextSize(20);
        phoneIntegrationTitle.setPadding(0, dp(24), 0, dp(8));
        layout.addView(phoneIntegrationTitle);

        Switch phoneIntegrationSwitch = new Switch(this);
        phoneIntegrationSwitch.setId(R.id.phone_integration);
        phoneIntegrationSwitch.setText("Register AnalogBridge calling account");
        phoneIntegrationSwitch.setChecked(phoneIntegration.isRegistered());
        phoneIntegrationSwitch.setOnCheckedChangeListener(
                new CompoundButton.OnCheckedChangeListener() {
                    @Override public void onCheckedChanged(
                            CompoundButton button, boolean checked) {
                        if (updatingPhoneIntegration) {
                            return;
                        }
                        try {
                            phoneIntegration.setRegistered(checked);
                            result.setText(checked
                                    ? "AnalogBridge calling account registered; enable it in Android settings"
                                    : "AnalogBridge calling account removed");
                        } catch (RuntimeException error) {
                            updatingPhoneIntegration = true;
                            button.setChecked(phoneIntegration.isRegistered());
                            updatingPhoneIntegration = false;
                            result.setText("Phone integration failed: " + safeMessage(error));
                        }
                    }
                });
        layout.addView(phoneIntegrationSwitch);

        Button phoneAccountSettings = new Button(this);
        phoneAccountSettings.setId(R.id.phone_account_settings);
        phoneAccountSettings.setText("Open calling account settings");
        phoneAccountSettings.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) {
                try {
                    phoneIntegration.openCallingAccountSettings(MainActivity.this);
                } catch (RuntimeException error) {
                    result.setText("Calling account settings unavailable");
                }
            }
        });
        layout.addView(phoneAccountSettings);

        result = new TextView(this);
        result.setId(R.id.result);
        result.setText("Not checked");
        result.setPadding(0, dp(20), 0, 0);
        layout.addView(result);

        setContentView(scroll);
    }

    private void saveEnrollment() {
        try {
            String endpointValue = endpoint.getText().toString().trim();
            String pinValue = certificatePin.getText().toString().trim();
            if ("https".equals(Endpoint.parse(endpointValue, "/api/v1/health").getProtocol())) {
                if (pinValue.isEmpty()) {
                    throw new IllegalArgumentException("HTTPS certificate pin is required");
                }
                CertificatePin.parse(pinValue);
            } else if (!pinValue.isEmpty()) {
                CertificatePin.parse(pinValue);
            }
            String tokenValue = token.getText().toString().trim();
            if (!tokenValue.isEmpty()) {
                vault.store(tokenValue);
            }
            settings.save(endpointValue, pinValue, tlsName.getText().toString().trim());
            token.setText("");
            result.setText("Enrollment saved securely");
        } catch (Exception error) {
            result.setText("Could not save enrollment: " + safeMessage(error));
        }
    }

    private void checkDaemon() {
        result.setText("Checking…");
        String endpointValue = endpoint.getText().toString();
        executor.execute(new Runnable() {
            @Override
            public void run() {
                String message;
                try {
                    ApiClient client = apiClient();
                    client.health(endpointValue);
                    String savedToken = vault.load();
                    if (savedToken.isEmpty()) {
                        message = "Daemon healthy; enrollment token not saved";
                    } else {
                        client.status(endpointValue, savedToken);
                        message = "Daemon healthy and client authenticated";
                    }
                } catch (Exception error) {
                    message = "Check failed: " + safeMessage(error);
                }
                final String finalMessage = message;
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        result.setText(finalMessage);
                    }
                });
            }
        });
    }

    private void confirmClearEnrollment() {
        new AlertDialog.Builder(this)
                .setTitle("Clear enrollment token?")
                .setMessage("The client will keep its endpoint and certificate pin but will no longer authenticate.")
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Clear", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        vault.clear();
                        token.setText("");
                        result.setText("Enrollment token cleared");
                    }
                })
                .show();
    }

    private void confirmSend() {
        final String recipientValue = recipient.getText().toString().trim();
        final String bodyValue = messageBody.getText().toString();
        if (recipientValue.isEmpty() || bodyValue.isEmpty()) {
            result.setText("Recipient and message are required");
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle("Send this SMS?")
                .setMessage("To: " + recipientValue + "\n\n" + bodyValue)
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Send", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface dialog, int which) {
                        sendMessage(recipientValue, bodyValue);
                    }
                })
                .show();
    }

    private void sendMessage(final String recipientValue, final String bodyValue) {
        result.setText("Sending…");
        final String endpointValue = endpoint.getText().toString();
        executor.execute(new Runnable() {
            @Override
            public void run() {
                String message;
                boolean sent = false;
                try {
                    apiClient().sendMessage(
                            endpointValue, vault.load(), recipientValue, bodyValue);
                    message = "Message accepted by iPhone transport";
                    sent = true;
                } catch (Exception error) {
                    message = "Send failed: " + safeMessage(error);
                }
                final String finalMessage = message;
                final boolean clearBody = sent;
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        result.setText(finalMessage);
                        if (clearBody) {
                            messageBody.setText("");
                        }
                    }
                });
            }
        });
    }

    private void confirmCallCommand(String title, String message, final String action,
            final String value) {
        new AlertDialog.Builder(this)
                .setTitle(title)
                .setMessage(message)
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Confirm", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface dialog, int which) {
                        executeCallCommand(action, value);
                    }
                })
                .show();
    }

    private void executeCallCommand(final String action, final String value) {
        result.setText("Sending call command…");
        final String endpointValue = endpoint.getText().toString();
        executor.execute(new Runnable() {
            @Override
            public void run() {
                String message;
                try {
                    apiClient().executeCallCommand(endpointValue, vault.load(), action, value);
                    message = "Call command accepted";
                } catch (Exception error) {
                    message = "Call command failed: " + safeMessage(error);
                }
                final String finalMessage = message;
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        result.setText(finalMessage);
                    }
                });
            }
        });
    }

    private void requestStartCallAudio() {
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO)
                != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.RECORD_AUDIO},
                    RECORD_AUDIO_PERMISSION_REQUEST);
            return;
        }
        startCallAudio();
    }

    private void startCallAudio() {
        final int generation;
        synchronized (audioLock) {
            if (audioSession != null) {
                result.setText("Call audio is already active");
                return;
            }
            generation = ++audioGeneration;
        }
        startAudio.setEnabled(false);
        stopAudio.setEnabled(true);
        result.setText("Starting call audio…");
        final String endpointValue = endpoint.getText().toString().trim();
        final String pinValue = certificatePin.getText().toString().trim();
        final String tlsNameValue = tlsName.getText().toString().trim();
        final boolean speakerphoneValue = speakerphone.isChecked();
        audioExecutor.execute(new Runnable() {
            @Override public void run() {
                AndroidCallAudioSession created = null;
                String message;
                boolean active = false;
                try {
                    String savedToken = vault.load();
                    MediaSessionCredentials credentials = new ApiClient(pinValue, tlsNameValue)
                            .createMediaSession(endpointValue, savedToken);
                    created = AndroidCallAudioSession.connect(getApplicationContext(),
                            endpointValue, pinValue, tlsNameValue, credentials);
                    created.setSpeakerphone(speakerphoneValue);
                    created.start();
                    synchronized (audioLock) {
                        if (generation == audioGeneration && audioSession == null) {
                            audioSession = created;
                            created = null;
                            active = true;
                        }
                    }
                    message = active ? "Call audio active" : "Call audio start cancelled";
                } catch (Exception error) {
                    message = "Call audio failed: " + safeMessage(error);
                } finally {
                    if (created != null) {
                        created.close();
                    }
                }
                final String finalMessage = message;
                final boolean finalActive = active;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        if (generation == audioGeneration && !isFinishing()) {
                            startAudio.setEnabled(!finalActive);
                            stopAudio.setEnabled(finalActive);
                            result.setText(finalMessage);
                            if (finalActive) {
                                mainHandler.removeCallbacks(audioHealthCheck);
                                mainHandler.postDelayed(audioHealthCheck, 500);
                            }
                        }
                    }
                });
            }
        });
    }

    private void setCallAudioSpeakerphone(final boolean enabled) {
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
                } catch (RuntimeException error) {
                    runOnUiThread(new Runnable() {
                        @Override public void run() {
                            if (!isFinishing()) {
                                result.setText("Call audio routing failed");
                            }
                        }
                    });
                }
            }
        });
    }

    private void stopCallAudio(final boolean report) {
        mainHandler.removeCallbacks(audioHealthCheck);
        final AndroidCallAudioSession stopped;
        synchronized (audioLock) {
            audioGeneration++;
            stopped = audioSession;
            audioSession = null;
        }
        startAudio.setEnabled(true);
        stopAudio.setEnabled(false);
        if (report) {
            result.setText("Stopping call audio…");
        }
        audioExecutor.execute(new Runnable() {
            @Override public void run() {
                if (stopped != null) {
                    stopped.close();
                }
                if (report) {
                    runOnUiThread(new Runnable() {
                        @Override public void run() {
                            if (!isFinishing()) {
                                result.setText("Call audio stopped");
                            }
                        }
                    });
                }
            }
        });
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions,
            int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != RECORD_AUDIO_PERMISSION_REQUEST) {
            return;
        }
        if (grantResults.length == 1
                && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
            startCallAudio();
        } else {
            result.setText("Microphone permission is required for call audio");
        }
    }

    private static String safeMessage(Exception error) {
        String message = error.getMessage();
        return message == null || message.trim().isEmpty() ? "unexpected error" : message;
    }

    private ApiClient apiClient() throws Exception {
        return new ApiClient(settings.certificatePin(), settings.tlsName());
    }

    private void discoverDaemon() {
        result.setText("Discovering…");
        discovery.discover(new NsdDiscovery.Callback() {
            @Override public void onResolved(String value, String identity) {
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        endpoint.setText(value);
                        tlsName.setText(identity);
                        settings.save(value, certificatePin.getText().toString().trim(), identity);
                        result.setText("Daemon discovered");
                    }
                });
            }
            @Override public void onFailure(final String reason) {
                runOnUiThread(new Runnable() {
                    @Override public void run() { result.setText("Daemon discovery failed: " + reason); }
                });
            }
        });
    }

    @Override
    protected void onStart() {
        super.onStart();
        discoverDaemon();
    }

    @Override
    protected void onStop() {
        mainHandler.removeCallbacks(audioHealthCheck);
        stopCallAudio(false);
        super.onStop();
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    @Override
    protected void onDestroy() {
        discovery.stop();
        mainHandler.removeCallbacks(audioHealthCheck);
        AndroidCallAudioSession stopped;
        synchronized (audioLock) {
            audioGeneration++;
            stopped = audioSession;
            audioSession = null;
        }
        if (stopped != null) {
            stopped.close();
        }
        audioExecutor.shutdown();
        executor.shutdownNow();
        super.onDestroy();
    }
}
