# Observed call-history semantics

Status: `UNKNOWN` / not yet exposed.

AnalogConnect must record only calls it observes after the feature is installed. It must not label
the result as iPhone call history. The current aggregate HFP adapter does not expose a trustworthy
remote number or sufficiently attributed multi-call transition stream, so persisting Recents now
would create misleading or unaddressable rows.

Required evidence before implementation: direction plus remote routing number attached to one
call lifecycle, monotonic transition identity, and explicit terminal cause where available.
Disposition must remain unknown when HFP cannot distinguish missed, declined, cancelled, or
failed. This is a backend evidence blocker, not a reason to fabricate UI data.
