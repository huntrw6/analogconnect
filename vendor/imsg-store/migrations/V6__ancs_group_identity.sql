-- Durable ANCS Subtitle identities remain separate from immutable message row IDs.
CREATE TABLE ancs_group_conversations (
    group_id          TEXT PRIMARY KEY,
    display_subtitle  TEXT NOT NULL,
    identity_source   TEXT NOT NULL CHECK (identity_source = 'ANCS_SUBTITLE_V1'),
    first_seen        INTEGER NOT NULL,
    last_seen         INTEGER NOT NULL,
    conversation_kind TEXT NOT NULL CHECK (conversation_kind = 'GROUP'),
    identity_conflict INTEGER NOT NULL DEFAULT 0 CHECK (identity_conflict IN (0, 1))
);

CREATE TABLE ancs_group_senders (
    group_id   TEXT NOT NULL REFERENCES ancs_group_conversations(group_id),
    sender     TEXT NOT NULL,
    first_seen INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL,
    PRIMARY KEY (group_id, sender)
);

CREATE TABLE ancs_group_message_assignments (
    map_handle       TEXT PRIMARY KEY REFERENCES messages(map_handle),
    group_id         TEXT NOT NULL REFERENCES ancs_group_conversations(group_id),
    notification_uid BLOB NOT NULL,
    correlated_at    INTEGER NOT NULL
);

-- Reserved now so a later rename/collision split can redirect identities without
-- rewriting immutable message row IDs.
CREATE TABLE ancs_group_identity_aliases (
    alias_group_id  TEXT PRIMARY KEY,
    target_group_id TEXT NOT NULL,
    relation        TEXT NOT NULL CHECK (relation IN ('alias', 'split')),
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_ancs_group_assignments_group
ON ancs_group_message_assignments (group_id);
