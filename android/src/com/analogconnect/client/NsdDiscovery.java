package com.analogconnect.client;

import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;

import java.net.Inet6Address;
import java.net.InetAddress;

final class NsdDiscovery {
    interface Callback {
        void onResolved(String endpoint, String tlsName);
        void onFailure();
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
            @Override public void onStartDiscoveryFailed(String type, int code) { stop(); callback.onFailure(); }
            @Override public void onStopDiscoveryFailed(String type, int code) { listener = null; }
            @Override public void onServiceFound(NsdServiceInfo service) {
                if (!service.getServiceType().equalsIgnoreCase(SERVICE_TYPE)) return;
                stop();
                manager.resolveService(service, new NsdManager.ResolveListener() {
                    @Override public void onResolveFailed(NsdServiceInfo info, int code) { callback.onFailure(); }
                    @Override public void onServiceResolved(NsdServiceInfo info) {
                        InetAddress host = info.getHost();
                        if (host == null || info.getPort() < 1 || info.getPort() > 65535) { callback.onFailure(); return; }
                        String address = host.getHostAddress();
                        if (host instanceof Inet6Address) address = "[" + address + "]";
                        callback.onResolved("https://" + address + ":" + info.getPort(), host.getHostName());
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
