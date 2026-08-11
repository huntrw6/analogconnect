# Vendored imsg extensions

`imsg-store` and `imsg-session` are derived from imsg 0.3.1
([upstream](https://github.com/gnufood/imsg), MIT license). They are kept narrow
so both `analogconnectd` and the locally built `imsg` CLI use the same encrypted
schema and ingestion model.

AnalogConnect changes preserve the complete MAP originator/recipient address set,
derive a sorted private conversation key, refresh existing handles after migration,
and query histories by that key. This fixes group messages being split into one
row per sender. Group reply remains disabled until multi-recipient push is
implemented and hardware-verified.

The MAP extension also implements the optional standardized conversation-listing
GET and an aggregate-only hardware probe. The tested iPhone accepts the request
but returns an empty list, so this remains a capability seam and diagnostic rather
than a basis for unsafe heuristic grouping.

The upstream license is in `vendor/IMSG-LICENSE`.
