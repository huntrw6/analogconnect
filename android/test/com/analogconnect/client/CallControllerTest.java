package com.analogconnect.client;

public final class CallControllerTest {
    public static void main(String[] args) {
        idleOnlyDials();
        incomingOffersAnswerAndReject();
        activeEnablesAudioHangupAndKeypad();
        terminalStatesPermitAnotherCall();
        validatesDtmfWithoutPrivateData();
        rejectsUnknownState();
        System.out.println("ANDROID_CALL_CONTROLLER_TESTS=PASS tests=6");
    }

    private static void idleOnlyDials() {
        CallController.State state = CallController.reduce("idle");
        require(state.canDial && !state.canAnswer && !state.canHangUp && !state.needsAudio,
                "idle controls");
    }

    private static void incomingOffersAnswerAndReject() {
        CallController.State state = CallController.reduce("incoming");
        require(state.canAnswer && state.canReject && !state.canHangUp && !state.needsAudio,
                "incoming controls");
    }

    private static void activeEnablesAudioHangupAndKeypad() {
        CallController.State state = CallController.reduce("active");
        require(state.canHangUp && state.canSendDtmf && state.needsAudio && !state.canDial,
                "active controls");
    }

    private static void terminalStatesPermitAnotherCall() {
        require(CallController.reduce("ended").canDial, "ended redial");
        require(CallController.reduce("error").canDial, "error recovery");
        require(CallController.reduce("dialing").canHangUp, "dialing cancellation");
        require(CallController.reduce("ringing").canHangUp, "ringing cancellation");
        require(!CallController.reduce("ending").canDial, "ending blocks controls");
        require(CallController.reduce("connection_lost").canDial, "connection recovery");
    }

    private static void validatesDtmfWithoutPrivateData() {
        for (String tone : new String[] {"0", "9", "*", "#", "A", "D"}) {
            require(CallController.validDtmf(tone), "valid DTMF");
        }
        for (String tone : new String[] {null, "", "12", "E", "a", "+"}) {
            require(!CallController.validDtmf(tone), "invalid DTMF");
        }
    }

    private static void rejectsUnknownState() {
        try {
            CallController.reduce("unknown");
            throw new AssertionError("unknown state accepted");
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
