package com.analogconnect.client;

final class CallMonitorTransition {
    private CallMonitorTransition() {}

    static boolean shouldShowIncoming(String previous, String current) {
        return "incoming".equals(current) && !"incoming".equals(previous);
    }

    static boolean shouldCancelIncoming(String previous, String current) {
        return "incoming".equals(previous) && !"incoming".equals(current);
    }
}
