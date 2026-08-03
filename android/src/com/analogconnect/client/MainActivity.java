package com.analogconnect.client;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.os.Bundle;
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
    private static final String DEFAULT_ENDPOINT = "http://127.0.0.1:8787";
    private static final String ENDPOINT_KEY = "endpoint";
    private static final String CERTIFICATE_PIN_KEY = "certificate_pin";
    private static final String TLS_NAME_KEY = "tls_name";

    private final ExecutorService executor = Executors.newSingleThreadExecutor();
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

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        vault = new TokenVault(this);
        discovery = new NsdDiscovery(this);

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
        endpoint.setText(getPreferences(MODE_PRIVATE).getString(ENDPOINT_KEY, DEFAULT_ENDPOINT));
        layout.addView(endpoint);

        Button discover = new Button(this);
        discover.setText("Discover daemon");
        discover.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { discoverDaemon(); }
        });
        layout.addView(discover);

        tlsName = new EditText(this);
        tlsName.setHint("TLS server name");
        tlsName.setSingleLine(true);
        tlsName.setText(getPreferences(MODE_PRIVATE).getString(TLS_NAME_KEY, ""));
        layout.addView(tlsName);

        certificatePin = new EditText(this);
        certificatePin.setId(R.id.certificate_pin);
        certificatePin.setHint("HTTPS certificate SHA-256 pin");
        certificatePin.setSingleLine(true);
        certificatePin.setText(getPreferences(MODE_PRIVATE)
                .getString(CERTIFICATE_PIN_KEY, ""));
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
            vault.store(token.getText().toString());
            getPreferences(MODE_PRIVATE).edit()
                    .putString(ENDPOINT_KEY, endpointValue)
                    .putString(CERTIFICATE_PIN_KEY, pinValue)
                    .putString(TLS_NAME_KEY, tlsName.getText().toString().trim())
                    .apply();
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

    private static String safeMessage(Exception error) {
        String message = error.getMessage();
        return message == null || message.trim().isEmpty() ? "unexpected error" : message;
    }

    private ApiClient apiClient() throws Exception {
        return new ApiClient(getPreferences(MODE_PRIVATE).getString(CERTIFICATE_PIN_KEY, ""),
                getPreferences(MODE_PRIVATE).getString(TLS_NAME_KEY, ""));
    }

    private void discoverDaemon() {
        result.setText("Discovering…");
        discovery.discover(new NsdDiscovery.Callback() {
            @Override public void onResolved(String value, String identity) {
                runOnUiThread(new Runnable() {
                    @Override public void run() { endpoint.setText(value); tlsName.setText(identity); result.setText("Daemon discovered; save enrollment"); }
                });
            }
            @Override public void onFailure() {
                runOnUiThread(new Runnable() {
                    @Override public void run() { result.setText("Daemon discovery failed"); }
                });
            }
        });
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    @Override
    protected void onDestroy() {
        discovery.stop();
        executor.shutdownNow();
        super.onDestroy();
    }
}
