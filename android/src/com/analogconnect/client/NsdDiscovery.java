package com.analogconnect.client;

import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;

import java.nio.charset.StandardCharsets;

final class NsdDiscovery {
    interface Callback {
        void onResolved(String endpoint, String tlsName);
        void onFailure(String reason);
    }

    private static final String SERVICE_TYPE = "_analogconnect._tcp.";
    private final NsdManager manager;
    private NsdManager.DiscoveryListener listener;

    NsdDiscovery(Context context) {
        manager = (NsdManager) context.getSystemService(Context.NSD_SERVICE);
    }

    void discover(final Callback callback) {
        stop();
        listener = new NsdManager.DiscoveryListener() {
            @Override public void onDiscoveryStarted(String type) {}
            @Override public void onDiscoveryStopped(String type) {}
            @Override public void onServiceLost(NsdServiceInfo service) {}
            @Override public void onStartDiscoveryFailed(String type, int code) { stop(); callback.onFailure("start_failed"); }
            @Override public void onStopDiscoveryFailed(String type, int code) { listener = null; }
            @Override public void onServiceFound(NsdServiceInfo service) {
                if (!service.getServiceType().equalsIgnoreCase(SERVICE_TYPE)) return;
                stop();
                manager.resolveService(service, new NsdManager.ResolveListener() {
                    @Override public void onResolveFailed(NsdServiceInfo info, int code) { callback.onFailure("resolve_failed"); }
                    @Override public void onServiceResolved(NsdServiceInfo info) {
                        try {
                            byte[] identity = info.getAttributes().get("tls-name");
                            DiscoveryTarget target = DiscoveryTarget.from(info.getHost(), info.getPort(),
                                    identity == null ? null : new String(identity, StandardCharsets.US_ASCII));
                            callback.onResolved(target.endpoint, target.tlsName);
                        } catch (IllegalArgumentException error) { callback.onFailure("identity_unavailable"); }
                    }
                });
            }
        };
        manager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener);
    }

    void stop() {
        if (listener != null) {
            NsdManager.DiscoveryListener active = listener;
            listener = null;
            try { manager.stopServiceDiscovery(active); } catch (IllegalArgumentException ignored) {}
        }
    }
}
