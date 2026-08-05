package com.analogconnect.client;

import android.accounts.Account;
import android.app.Service;
import android.content.AbstractThreadedSyncAdapter;
import android.content.ContentProviderClient;
import android.content.Context;
import android.content.Intent;
import android.content.SyncResult;
import android.os.Bundle;
import android.os.IBinder;

public final class AnalogContactSyncService extends Service {
    private SyncAdapter adapter;

    @Override public void onCreate() {
        adapter = new SyncAdapter(this);
    }

    @Override public IBinder onBind(Intent intent) {
        return adapter.getSyncAdapterBinder();
    }

    private static final class SyncAdapter extends AbstractThreadedSyncAdapter {
        SyncAdapter(Context context) {
            super(context, false);
        }

        @Override public void onPerformSync(Account account, Bundle extras, String authority,
                ContentProviderClient provider, SyncResult result) {
            // Fail closed until the authenticated contact snapshot contract is implemented.
            result.stats.numSkippedEntries++;
        }
    }
}
