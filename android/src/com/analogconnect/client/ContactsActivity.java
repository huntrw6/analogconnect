package com.analogconnect.client;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.ArrayList;
import java.util.List;

public final class ContactsActivity extends Activity implements ContactController.View {
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private LinearLayout content;
    private EditText search;
    private EnrollmentSettings settings;
    private TokenVault vault;
    private ContactController.Runner controller;
    private final List<ContactListItem> visibleContacts = new ArrayList<ContactListItem>();
    private String currentQuery = "";
    private String requestedCursor;
    private String nextCursor;
    private boolean loading;
    private OfflineCache offlineCache;
    private volatile boolean showingCachedData;

    @Override protected void onCreate(Bundle state) {
        Ui.applyTheme(this);
        super.onCreate(state);
        settings = new EnrollmentSettings(this);
        vault = new TokenVault(this);
        offlineCache = new OfflineCache(this);

        content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(20);
        content.setPadding(padding, padding, padding, padding);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(content);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 1f));
        root.addView(Ui.bottomNavigation(this, "Contacts"));
        setContentView(root);

        controller = new ContactController.Runner(new ContactController.Gateway() {
            @Override public ConversationPageData<ContactListItem> contacts(
                    String query, String cursor)
                    throws Exception {
                if (settings.demoMode()) return DemoFixtures.contacts(query);
                try {
                    ApiClient client = new ApiClient(settings.certificatePin(), settings.tlsName());
                    ConversationPageData<ContactListItem> page = client.contacts(
                            settings.endpoint(), vault.load(), query, cursor);
                    if (cursor == null && (query == null || query.trim().isEmpty())) {
                        offlineCache.storeContacts(page);
                    }
                    showingCachedData = false;
                    return page;
                } catch (Exception error) {
                    ConversationPageData<ContactListItem> cached = offlineCache.loadContacts(query);
                    if (cached == null) throw error;
                    showingCachedData = true;
                    return cached;
                }
            }
        }, this);
        showSearchChrome();
        loadContacts("", null);
    }

    private void showSearchChrome() {
        String previousQuery = search == null ? "" : search.getText().toString();
        content.removeAllViews();
        Button back = new Button(this);
        back.setText("Back");
        back.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) { finish(); }
        });
        content.addView(back);
        content.addView(text("Contacts", 28));
        search = new EditText(this);
        search.setHint("Search contact names");
        search.setSingleLine(true);
        search.setInputType(InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_FLAG_CAP_WORDS);
        search.setText(previousQuery);
        content.addView(search);
        Button searchButton = new Button(this);
        searchButton.setText("Search");
        searchButton.setOnClickListener(new View.OnClickListener() {
            @Override public void onClick(View view) {
                loadContacts(search.getText().toString(), null);
            }
        });
        content.addView(searchButton);
    }

    private void loadContacts(final String query, final String cursor) {
        if (loading) {
            return;
        }
        loading = true;
        currentQuery = query == null ? "" : query.trim();
        requestedCursor = cursor;
        executor.execute(new Runnable() {
            @Override public void run() { controller.load(query, cursor); }
        });
    }

    @Override public void showLoading() {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                status("Loading contacts…");
            }
        });
    }

    @Override public void showContacts(final ConversationPageData<ContactListItem> page) {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                loading = false;
                showSearchChrome();
                if (showingCachedData) status("iPhone disconnected · showing saved contacts");
                if (requestedCursor == null) {
                    visibleContacts.clear();
                }
                visibleContacts.addAll(page.items);
                nextCursor = page.nextCursor;
                if (visibleContacts.isEmpty()) {
                    content.addView(text("No matching synchronized contacts.", 18));
                    return;
                }
                for (ContactListItem contact : visibleContacts) {
                    for (final String number : contact.phoneNumbers) {
                        LinearLayout row = new LinearLayout(ContactsActivity.this);
                        row.setOrientation(LinearLayout.VERTICAL);
                        row.setPadding(0, dp(8), 0, dp(8));
                        TextView label = text(contact.labelFor(number), 18);
                        if (contact.displayName != null) label.setTypeface(null,
                                android.graphics.Typeface.BOLD);
                        row.addView(label);
                        LinearLayout actions = new LinearLayout(ContactsActivity.this);
                        Button callButton = new Button(ContactsActivity.this);
                        callButton.setText("Call");
                        callButton.setContentDescription(contact.displayName == null
                                ? "Call contact number" : "Call " + contact.displayName);
                        callButton.setOnClickListener(new View.OnClickListener() {
                            @Override public void onClick(View view) {
                                Intent call = new Intent(ContactsActivity.this,
                                        CallsActivity.class);
                                call.putExtra(CallsActivity.EXTRA_DIAL_TARGET, number);
                                if (contact.displayName != null) call.putExtra(
                                        CallsActivity.EXTRA_DISPLAY_NAME, contact.displayName);
                                startActivity(call);
                            }
                        });
                        actions.addView(callButton, new LinearLayout.LayoutParams(0, dp(48), 1f));
                        Button messageButton = new Button(ContactsActivity.this);
                        messageButton.setText("Message");
                        messageButton.setContentDescription("Message contact privately");
                        messageButton.setOnClickListener(new View.OnClickListener() {
                            @Override public void onClick(View view) {
                                Intent message = new Intent(ContactsActivity.this,
                                        ComposeActivity.class);
                                message.putExtra(ComposeActivity.EXTRA_RECIPIENT, number);
                                startActivity(message);
                            }
                        });
                        actions.addView(messageButton,
                                new LinearLayout.LayoutParams(0, dp(48), 1f));
                        row.addView(actions);
                        content.addView(row);
                    }
                }
                if (nextCursor != null) {
                    Button more = new Button(ContactsActivity.this);
                    more.setText("Load more contacts");
                    more.setOnClickListener(new View.OnClickListener() {
                        @Override public void onClick(View view) {
                            loadContacts(currentQuery, nextCursor);
                        }
                    });
                    content.addView(more);
                }
            }
        });
    }

    @Override public void showFixedError() {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                loading = false;
                status("Could not load contacts · try again");
            }
        });
    }

    private void status(String value) {
        TextView status = text(value, 17);
        status.setContentDescription(value);
        content.addView(status);
    }

    private TextView text(String value, int size) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(size);
        return view;
    }

    @Override protected void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
