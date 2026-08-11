package com.analogconnect.client;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;

import org.json.JSONArray;
import org.json.JSONObject;

import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import java.util.ArrayList;
import java.util.List;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** Small encrypted last-known-good cache. It is never consulted for send routing or authority. */
final class OfflineCache {
    private static final String ALIAS = "analogconnect.offline.cache";
    private static final String PREFS = "offline_cache";
    private final SharedPreferences preferences;

    OfflineCache(Context context) {
        preferences = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    void storeConversations(ConversationPageData<ConversationSummary> page) {
        try {
            JSONArray items = new JSONArray();
            for (ConversationSummary item : page.items) {
                JSONObject value = new JSONObject();
                value.put("id", item.id).put("address", item.displayAddress)
                        .put("name", item.displayName).put("group", item.group)
                        .put("reply", item.replySupported).put("time", item.latestUnixMillis)
                        .put("count", item.messageCount).put("unread", item.unreadCount)
                        .put("state", item.latestOutgoingState).put("kind", item.kind)
                        .put("title", item.title).put("can_reply", item.canReply)
                        .put("conflict", item.identityConflict).put("preview", item.latestPreview)
                        .put("sender", item.latestSender).put("sent", item.latestSent);
                items.put(value);
            }
            store("conversations", items.toString());
        } catch (Exception ignored) { }
    }

    ConversationPageData<ConversationSummary> loadConversations() {
        try {
            JSONArray items = new JSONArray(load("conversations"));
            List<ConversationSummary> result = new ArrayList<ConversationSummary>();
            for (int index = 0; index < items.length(); index++) {
                JSONObject value = items.getJSONObject(index);
                result.add(new ConversationSummary(value.getString("id"),
                        value.getString("address"), nullable(value, "name"),
                        value.getBoolean("group"), value.getBoolean("reply"),
                        value.getLong("time"), value.getLong("count"), value.getLong("unread"),
                        nullable(value, "state"), value.getString("kind"), value.getString("title"),
                        value.getBoolean("can_reply"), value.getBoolean("conflict"),
                        value.getString("preview"), value.getString("sender"),
                        value.getBoolean("sent")));
            }
            return new ConversationPageData<ConversationSummary>(result, null);
        } catch (Exception ignored) { return null; }
    }

    void storeMessages(String id, ConversationPageData<ConversationMessage> page) {
        try {
            JSONArray items = new JSONArray();
            for (ConversationMessage item : page.items) {
                items.put(new JSONObject().put("id", item.id).put("time", item.timestampUnixMillis)
                        .put("direction", item.sent ? "sent" : "received")
                        .put("peer", item.peerAddress).put("body", item.body)
                        .put("read", item.read).put("state", item.outgoingState));
            }
            store("messages_" + id, items.toString());
        } catch (Exception ignored) { }
    }

    ConversationPageData<ConversationMessage> loadMessages(String id) {
        try {
            JSONArray items = new JSONArray(load("messages_" + id));
            List<ConversationMessage> result = new ArrayList<ConversationMessage>();
            for (int index = 0; index < items.length(); index++) {
                JSONObject value = items.getJSONObject(index);
                result.add(new ConversationMessage(value.getString("id"), value.getLong("time"),
                        value.getString("direction"), value.getString("peer"),
                        value.getString("body"), value.getBoolean("read"),
                        nullable(value, "state")));
            }
            return new ConversationPageData<ConversationMessage>(result, null);
        } catch (Exception ignored) { return null; }
    }

    void storeContacts(ConversationPageData<ContactListItem> page) {
        try {
            JSONArray items = new JSONArray();
            for (ContactListItem item : page.items) {
                items.put(new JSONObject().put("name", item.displayName)
                        .put("phones", new JSONArray(item.phoneNumbers)));
            }
            store("contacts", items.toString());
        } catch (Exception ignored) { }
    }

    ConversationPageData<ContactListItem> loadContacts(String query) {
        try {
            JSONArray items = new JSONArray(load("contacts"));
            String needle = query == null ? "" : query.trim().toLowerCase(java.util.Locale.ROOT);
            List<ContactListItem> result = new ArrayList<ContactListItem>();
            for (int index = 0; index < items.length(); index++) {
                JSONObject value = items.getJSONObject(index);
                String name = nullable(value, "name");
                JSONArray phones = value.getJSONArray("phones");
                List<String> numbers = new ArrayList<String>();
                for (int phone = 0; phone < phones.length(); phone++) numbers.add(phones.getString(phone));
                if (needle.isEmpty() || name != null
                        && name.toLowerCase(java.util.Locale.ROOT).contains(needle)) {
                    result.add(new ContactListItem(name, numbers));
                }
            }
            return new ConversationPageData<ContactListItem>(result, null);
        } catch (Exception ignored) { return null; }
    }

    private static String nullable(JSONObject value, String key) {
        return value.isNull(key) ? null : value.optString(key, null);
    }

    private void store(String name, String clear) throws Exception {
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key());
        byte[] encrypted = cipher.doFinal(clear.getBytes(StandardCharsets.UTF_8));
        preferences.edit().putString(name, Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP)
                + ":" + Base64.encodeToString(encrypted, Base64.NO_WRAP)).apply();
    }

    private String load(String name) throws Exception {
        String blob = preferences.getString(name, null);
        if (blob == null) throw new IllegalStateException("No cache");
        String[] pieces = blob.split(":", 2);
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE, key(),
                new GCMParameterSpec(128, Base64.decode(pieces[0], Base64.NO_WRAP)));
        return new String(cipher.doFinal(Base64.decode(pieces[1], Base64.NO_WRAP)),
                StandardCharsets.UTF_8);
    }

    private SecretKey key() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        if (!store.containsAlias(ALIAS)) {
            KeyGenerator generator = KeyGenerator.getInstance(
                    KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(ALIAS,
                    KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());
            generator.generateKey();
        }
        return (SecretKey) store.getKey(ALIAS, null);
    }
}
