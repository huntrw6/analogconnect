package com.analogconnect.client;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.os.Bundle;
import android.text.InputType;
import android.text.Editable;
import android.text.TextWatcher;
import android.view.View;
import android.view.Gravity;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.List;
import java.text.DateFormat;
import java.util.Date;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class ConversationsActivity extends Activity implements ConversationController.View {
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private final MessageSendDraft sendDraft = new MessageSendDraft();
    private LinearLayout content;
    private EnrollmentSettings settings;
    private TokenVault vault;
    private ConversationController.Runner controller;
    private ConversationSummary currentConversation;
    private EditText composeBody;
    private Button sendButton;

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        settings = new EnrollmentSettings(this);
        vault = new TokenVault(this);

        content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(20);
        content.setPadding(padding, padding, padding, padding);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(content);
        setContentView(scroll);

        controller = new ConversationController.Runner(new ConversationController.Gateway() {
            @Override public ConversationPageData<ConversationSummary> conversations()
                    throws Exception {
                if (settings.demoMode()) return DemoFixtures.conversations();
                return apiClient().conversations(settings.endpoint(), vault.load());
            }

            @Override public ConversationPageData<ConversationMessage> messages(
                    String conversationId) throws Exception {
                if (settings.demoMode()) return DemoFixtures.messages(currentConversation);
                return apiClient().conversationMessages(
                        settings.endpoint(), vault.load(), conversationId);
            }
        }, this);
        refreshConversations();
    }

    private ApiClient apiClient() throws Exception {
        return new ApiClient(settings.certificatePin(), settings.tlsName());
    }

    private void refreshConversations() {
        executor.execute(new Runnable() {
            @Override public void run() {
                controller.loadConversations();
            }
        });
    }

    private void openConversation(final ConversationSummary conversation) {
        currentConversation = conversation;
        executor.execute(new Runnable() {
            @Override public void run() {
                controller.openConversation(conversation);
            }
        });
    }

    @Override public void showLoading() {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                content.removeAllViews();
                TextView loading = text("Loading…", 20);
                loading.setContentDescription("Loading conversations");
                content.addView(loading);
            }
        });
    }

    @Override public void showConversations(final ConversationPageData<ConversationSummary> page) {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                currentConversation = null;
                content.removeAllViews();
                LinearLayout toolbar = new LinearLayout(ConversationsActivity.this);
                toolbar.setGravity(Gravity.CENTER_VERTICAL);
                TextView heading = text("Messages", 30);
                heading.setTypeface(null, Typeface.BOLD);
                toolbar.addView(heading, new LinearLayout.LayoutParams(0, dp(56), 1f));
                Button compose = new Button(ConversationsActivity.this);
                compose.setText("New");
                compose.setContentDescription("New private message");
                compose.setOnClickListener(new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        startActivity(new android.content.Intent(ConversationsActivity.this,
                                ComposeActivity.class));
                    }
                });
                toolbar.addView(compose);
                content.addView(toolbar);
                final EditText search = new EditText(ConversationsActivity.this);
                search.setHint("Search conversations");
                search.setSingleLine(true);
                search.setContentDescription("Search conversation names and group titles");
                content.addView(search);
                Button refresh = new Button(ConversationsActivity.this);
                refresh.setText("Refresh");
                refresh.setOnClickListener(new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        refreshConversations();
                    }
                });
                content.addView(refresh);
                if (page.items.isEmpty()) {
                    content.addView(text("No synchronized conversations yet.", 18));
                    return;
                }
                for (final ConversationSummary conversation : page.items) {
                    LinearLayout row = new LinearLayout(ConversationsActivity.this);
                    row.setOrientation(LinearLayout.HORIZONTAL);
                    row.setGravity(Gravity.CENTER_VERTICAL);
                    row.setPadding(0, dp(10), 0, dp(10));
                    row.setMinimumHeight(dp(72));
                    TextView avatar = Ui.avatar(ConversationsActivity.this,
                            conversation.displayLabel(), conversation.group ? Ui.GREEN : Ui.BLUE);
                    row.addView(avatar, new LinearLayout.LayoutParams(dp(48), dp(48)));
                    LinearLayout labels = new LinearLayout(ConversationsActivity.this);
                    labels.setOrientation(LinearLayout.VERTICAL);
                    labels.setPadding(dp(14), 0, dp(8), 0);
                    TextView label = text(conversation.displayLabel(), 18);
                    TextView preview = text(conversation.group
                            ? "Group conversation" : "Private conversation", 15);
                    preview.setTextColor(Ui.MUTED);
                    if (conversation.unreadCount > 0) {
                        label.setTypeface(null, Typeface.BOLD);
                        preview.setTypeface(null, Typeface.BOLD);
                    }
                    labels.addView(label);
                    labels.addView(preview);
                    row.addView(labels, new LinearLayout.LayoutParams(0, -2, 1f));
                    TextView time = text(DateFormat.getDateTimeInstance(DateFormat.SHORT,
                            DateFormat.SHORT).format(new Date(conversation.latestUnixMillis)), 12);
                    time.setTextColor(Ui.MUTED);
                    row.addView(time);
                    String unread = conversation.unreadCount > 0
                            ? ", " + conversation.unreadCount + " unread" : "";
                    row.setContentDescription("Open conversation with "
                            + conversation.displayLabel() + unread);
                    row.setFocusable(true);
                    row.setClickable(true);
                    row.setTag(conversation.displayLabel().toLowerCase(Locale.ROOT));
                    row.setOnClickListener(new View.OnClickListener() {
                        @Override public void onClick(View view) {
                            openConversation(conversation);
                        }
                    });
                    content.addView(row);
                }
                search.addTextChangedListener(new TextWatcher() {
                    @Override public void beforeTextChanged(CharSequence text, int start,
                            int count, int after) { }
                    @Override public void onTextChanged(CharSequence text, int start,
                            int before, int count) {
                        String query = text.toString().trim().toLowerCase(Locale.ROOT);
                        for (int index = 0; index < content.getChildCount(); index++) {
                            View child = content.getChildAt(index);
                            Object tag = child.getTag();
                            if (tag instanceof String) child.setVisibility(
                                    ((String) tag).contains(query) ? View.VISIBLE : View.GONE);
                        }
                    }
                    @Override public void afterTextChanged(Editable text) { }
                });
                if (page.nextCursor != null) {
                    content.addView(text("More conversations are available; pagination UI pending.",
                            14));
                }
                content.addView(Ui.bottomNavigation(ConversationsActivity.this, "Messages"));
            }
        });
    }

    @Override public void showMessages(final ConversationSummary conversation,
            final ConversationPageData<ConversationMessage> page) {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                content.removeAllViews();
                Button back = new Button(ConversationsActivity.this);
                back.setText("‹ Messages");
                back.setOnClickListener(new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        refreshConversations();
                    }
                });
                content.addView(back);
                TextView threadTitle = text(conversation.displayLabel(), 24);
                threadTitle.setGravity(Gravity.CENTER_HORIZONTAL);
                threadTitle.setTypeface(null, Typeface.BOLD);
                content.addView(threadTitle);

                List<ConversationMessage> messages = page.items;
                for (int index = messages.size() - 1; index >= 0; index--) {
                    ConversationMessage message = messages.get(index);
                    String owner = message.sent ? ""
                            : (conversation.group && !message.peerAddress.isEmpty()
                                    ? message.peerAddress + "\n" : "");
                    String state = message.outgoingState == null
                            ? "" : "\nStatus: " + message.outgoingState.replace('_', ' ');
                    TextView bubble = text(owner + message.body + state, 18);
                    bubble.setPadding(dp(14), dp(10), dp(14), dp(10));
                    GradientDrawable bubbleBackground = new GradientDrawable();
                    bubbleBackground.setCornerRadius(dp(18));
                    bubbleBackground.setColor(message.sent ? 0xffd8eaff : 0xffeeeeee);
                    bubble.setBackground(bubbleBackground);
                    LinearLayout.LayoutParams bubbleParams = new LinearLayout.LayoutParams(
                            -2, -2);
                    bubbleParams.gravity = message.sent ? Gravity.RIGHT : Gravity.LEFT;
                    bubbleParams.setMargins(0, dp(4), 0, dp(4));
                    bubble.setContentDescription((message.sent ? "Sent message: "
                            : "Received message: ") + message.body + state);
                    content.addView(bubble, bubbleParams);
                }
                if (messages.isEmpty()) {
                    content.addView(text("No messages in this conversation.", 18));
                }
                if (page.nextCursor != null) {
                    content.addView(text("Older messages are available; pagination UI pending.", 14));
                }

                if (!conversation.canUsePrivateReply()) {
                    content.addView(text(
                            conversation.identityConflict
                                    ? "This conversation could not be identified safely. Reply is unavailable."
                                    : "Group replies aren't available yet. You can open a sender's "
                                            + "contact to message them privately.",
                            16));
                    return;
                }

                composeBody = new EditText(ConversationsActivity.this);
                composeBody.setHint("Message");
                composeBody.setMaxLines(5);
                composeBody.setInputType(InputType.TYPE_CLASS_TEXT
                        | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
                        | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
                content.addView(composeBody);
                sendButton = new Button(ConversationsActivity.this);
                sendButton.setText("Send");
                sendButton.setOnClickListener(new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        confirmSend(conversation);
                    }
                });
                content.addView(sendButton);
            }
        });
    }

    private void confirmSend(final ConversationSummary conversation) {
        if (!conversation.canUsePrivateReply()) {
            return;
        }
        final String body = composeBody.getText().toString();
        if (body.isEmpty()) {
            composeBody.setError("Message is required");
            return;
        }
        final String operationId = sendDraft.operationIdFor(conversation.displayAddress, body);
        new AlertDialog.Builder(this)
                .setTitle("Send a private message?")
                .setMessage(body)
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Send", new DialogInterface.OnClickListener() {
                    @Override public void onClick(DialogInterface dialog, int which) {
                        sendConfirmed(conversation, body, operationId);
                    }
                })
                .show();
    }

    private void sendConfirmed(final ConversationSummary conversation, final String body,
            final String operationId) {
        sendButton.setEnabled(false);
        sendButton.setText("Sending…");
        executor.execute(new Runnable() {
            @Override public void run() {
                boolean accepted = false;
                try {
                    apiClient().sendMessage(settings.endpoint(), vault.load(),
                            conversation.displayAddress, body, operationId);
                    accepted = true;
                } catch (Exception error) {
                    // UI receives a fixed failure below; private backend details are discarded.
                }
                final boolean finalAccepted = accepted;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        if (finalAccepted) {
                            sendDraft.accepted();
                            composeBody.setText("");
                            openConversation(conversation);
                        } else {
                            sendButton.setEnabled(true);
                            sendButton.setText("Retry review and send");
                            composeBody.setError("Send failed; draft preserved");
                        }
                    }
                });
            }
        });
    }

    @Override public void showFixedError(final String message) {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                content.removeAllViews();
                content.addView(text(message, 20));
                Button retry = new Button(ConversationsActivity.this);
                retry.setText(currentConversation == null ? "Retry conversations" : "Retry messages");
                retry.setOnClickListener(new View.OnClickListener() {
                    @Override public void onClick(View view) {
                        if (currentConversation == null) {
                            refreshConversations();
                        } else {
                            openConversation(currentConversation);
                        }
                    }
                });
                content.addView(retry);
            }
        });
    }

    private TextView text(String value, int size) {
        TextView text = new TextView(this);
        text.setText(value);
        text.setTextSize(size);
        return text;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    @Override protected void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }
}
