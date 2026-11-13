ALTER TABLE worlds
  ADD COLUMN size_category TEXT NOT NULL DEFAULT 'small'
    CHECK (size_category IN ('small', 'medium', 'large')),
  ADD COLUMN width INTEGER NOT NULL DEFAULT 32 CHECK (width > 0),
  ADD COLUMN height INTEGER NOT NULL DEFAULT 64 CHECK (height > 0),
  ADD COLUMN tick_duration_ms INTEGER NOT NULL DEFAULT 1000
    CHECK (tick_duration_ms BETWEEN 10 AND 60000);
