package com.analogconnect.client;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class ComposeActivity extends Activity {
    static final String EXTRA_RECIPIENT = "com.analogconnect.client.PRIVATE_RECIPIENT";
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private final MessageSendDraft draft = new MessageSendDraft();
    private EditText recipient;
    private EditText body;
    private Button send;

    @Override protected void onCreate(Bundle state) {
        Ui.applyTheme(this);
        super.onCreate(state);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(Ui.dp(this, 20), Ui.dp(this, 24), Ui.dp(this, 20), Ui.dp(this, 20));
        content.addView(Ui.button(this, "‹ Messages", new View.OnClickListener() {
            @Override public void onClick(View view) { finish(); }
        }));
        TextView title = Ui.text(this, "New message", 28);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        content.addView(title);
        recipient = new EditText(this);
        recipient.setId(R.id.compose_recipient);
        recipient.setHint("To: name or phone number");
        recipient.setInputType(InputType.TYPE_CLASS_PHONE);
        recipient.setSingleLine(true);
        recipient.setContentDescription("Private message recipient");
        String initialRecipient = getIntent().getStringExtra(EXTRA_RECIPIENT);
        if (initialRecipient != null && initialRecipient.length() <= 128) {
            recipient.setText(initialRecipient);
        }
        content.addView(recipient);
        TextView note = Ui.text(this, "New messages are private and sent to one recipient.", 14);
        note.setTextColor(Ui.mutedColor(this));
        content.addView(note);
        body = new EditText(this);
        body.setId(R.id.compose_body);
        body.setHint("Text message");
        body.setMinLines(3);
        body.setMaxLines(8);
        body.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
                | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        content.addView(body);
        send = Ui.button(this, "Send", new View.OnClickListener() {
            @Override public void onClick(View view) { review(); }
        });
        content.addView(send);
        setContentView(content);
    }

    private void review() {
        final String target = recipient.getText().toString().trim();
        final String message = body.getText().toString();
        if (target.isEmpty()) { recipient.setError("Choose a recipient"); return; }
        if (message.trim().isEmpty()) { body.setError("Write a message"); return; }
        final String operation = draft.operationIdFor(target, message);
        new AlertDialog.Builder(this).setTitle("Send this private message?")
                .setMessage(message).setNegativeButton("Cancel", null)
                .setPositiveButton("Send", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        send(target, message, operation);
                    }
                }).show();
    }

    private void send(final String target, final String message, final String operation) {
        send.setEnabled(false);
        send.setText("Sending…");
        executor.execute(new Runnable() {
            @Override public void run() {
                boolean accepted = false;
                try {
                    EnrollmentSettings settings = new EnrollmentSettings(ComposeActivity.this);
                    ApiClient client = new ApiClient(settings.certificatePin(), settings.tlsName());
                    client.sendMessage(settings.endpoint(), new TokenVault(ComposeActivity.this).load(),
                            target, message, operation);
                    accepted = true;
                } catch (Exception ignored) { }
                final boolean result = accepted;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        if (result) { draft.accepted(); finish(); }
                        else {
                            send.setEnabled(true);
                            send.setText("Try again");
                            body.setError("Message wasn't sent. Your draft is still here.");
                        }
                    }
                });
            }
        });
    }

    @Override protected void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }
}
