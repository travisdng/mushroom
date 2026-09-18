-- Notes the user has marked as never to be sent to an AI endpoint.
--
-- Derived from the Markdown like everything else here: it is read from the
-- note's own frontmatter (`ai: false` or `private: true`) at index time, so a
-- rebuilt index reproduces it exactly and deleting this database still loses
-- nothing.
--
-- It lives in the index rather than being checked at query time because the
-- retriever applies LIMIT in SQL: filtering afterwards would let excluded
-- notes eat result slots and quietly shrink the answer. Folder and glob rules
-- come from settings, can change without any note changing, and are therefore
-- applied in Rust after the fetch — the retriever already over-fetches for the
-- per-note cap, which gives that filtering its headroom.

ALTER TABLE notes ADD COLUMN ai_excluded INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_notes_ai_excluded ON notes(ai_excluded) WHERE ai_excluded = 1;

-- Existing rows default to 0, and 0 means "safe to send" — the unsafe
-- direction. A note already carrying `ai: false` would be sent exactly once,
-- on the first launch after upgrading, which is the one launch where the user
-- has most reason to believe they are protected.
--
-- So every row is marked as needing a re-index. `reconcile` treats a note as
-- unchanged when both `modified_at` and `size_bytes` match the file; -1 can
-- never be a real size, so every note is read again and its frontmatter
-- re-parsed on the next startup. `size_bytes` rather than `modified_at`
-- because modified_at also drives list ordering, and a failed reconcile would
-- leave that visibly wrong.
--
-- The alternative — defaulting the column to 1 — fails safe but hides every
-- note from the AI until a rebuild finishes, which looks like a broken product
-- and teaches people to distrust the feature.
UPDATE notes SET size_bytes = -1;
