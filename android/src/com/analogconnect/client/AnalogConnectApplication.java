package com.analogconnect.client;

import android.app.Application;

public final class AnalogConnectApplication extends Application {
    @Override public void onCreate() {
        super.onCreate();
        CallStateMonitorService.start(this);
    }
}
