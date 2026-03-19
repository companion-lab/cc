-- Initial schema for cc

CREATE TABLE IF NOT EXISTS sessions (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL,
    project_id          TEXT NOT NULL,
    directory           TEXT NOT NULL,
    title               TEXT NOT NULL,
    version             TEXT NOT NULL DEFAULT '1',
    parent_id           TEXT REFERENCES sessions(id),
    time_created        INTEGER NOT NULL,
    time_updated        INTEGER NOT NULL,
    summary_additions   INTEGER,
    summary_deletions   INTEGER,
    summary_files       INTEGER,
    summary_diffs       TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role         TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    time_created INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS parts (
    id          TEXT PRIMARY KEY,
    message_id  TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL, -- 'text' | 'tool_call' | 'tool_result' | 'reasoning'
    content     TEXT NOT NULL, -- JSON blob
    order_idx   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS todos (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    content    TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'pending',
    priority   TEXT NOT NULL DEFAULT 'medium',
    order_idx  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_parts_message    ON parts(message_id);
CREATE INDEX IF NOT EXISTS idx_todos_session    ON todos(session_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(time_updated DESC);
