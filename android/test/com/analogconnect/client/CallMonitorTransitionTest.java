package com.analogconnect.client;

public final class CallMonitorTransitionTest {
    public static void main(String[] args) {
        require(CallMonitorTransition.shouldShowIncoming("idle", "incoming"), "idle incoming");
        require(!CallMonitorTransition.shouldShowIncoming("incoming", "incoming"), "no duplicate");
        require(CallMonitorTransition.shouldCancelIncoming("incoming", "active"), "answer");
        require(CallMonitorTransition.shouldCancelIncoming("incoming", "idle"), "reject");
        require(!CallMonitorTransition.shouldCancelIncoming("active", "idle"), "no stale cancel");
        System.out.println("ANDROID_CALL_MONITOR_TESTS=PASS tests=5");
    }

    private static void require(boolean value, String label) {
        if (!value) throw new AssertionError(label);
    }
}
