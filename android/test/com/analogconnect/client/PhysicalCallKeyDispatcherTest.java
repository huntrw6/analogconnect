package com.analogconnect.client;

public final class PhysicalCallKeyDispatcherTest {
    public static void main(String[] args) {
        expect("incoming", 5, PhysicalCallKeyDispatcher.Action.ANSWER, "");
        expect("incoming", 6, PhysicalCallKeyDispatcher.Action.REJECT, "");
        expect("dialing", 6, PhysicalCallKeyDispatcher.Action.HANG_UP, "");
        expect("active", 6, PhysicalCallKeyDispatcher.Action.HANG_UP, "");
        expect("incoming", 26, PhysicalCallKeyDispatcher.Action.REJECT, "");
        expect("active", 26, PhysicalCallKeyDispatcher.Action.HANG_UP, "");
        expect("ending", 6, PhysicalCallKeyDispatcher.Action.NONE, "");
        expect("idle", 6, PhysicalCallKeyDispatcher.Action.NONE, "");
        expect("ended", 5, PhysicalCallKeyDispatcher.Action.NONE, "");
        expect("active", 12, PhysicalCallKeyDispatcher.Action.DTMF, "5");
        expect("active", 17, PhysicalCallKeyDispatcher.Action.DTMF, "*");
        expect("active", 18, PhysicalCallKeyDispatcher.Action.DTMF, "#");
        expect("idle", 12, PhysicalCallKeyDispatcher.Action.NONE, "");
        require(PhysicalCallKeyDispatcher.dispatch("incoming", 5, true, 1).action
                == PhysicalCallKeyDispatcher.Action.NONE, "repeat ignored");
        require(PhysicalCallKeyDispatcher.dispatch("incoming", 5, false, 0).action
                == PhysicalCallKeyDispatcher.Action.NONE, "key up ignored");
        expect("idle", 26, PhysicalCallKeyDispatcher.Action.NONE, "");
        System.out.println("ANDROID_PHYSICAL_CALL_KEY_TESTS=PASS tests=16");
    }

    private static void expect(String state, int key, PhysicalCallKeyDispatcher.Action action,
            String value) {
        PhysicalCallKeyDispatcher.Decision result =
                PhysicalCallKeyDispatcher.dispatch(state, key, true, 0);
        require(result.action == action && value.equals(result.value), state + " key " + key);
    }

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }
}
