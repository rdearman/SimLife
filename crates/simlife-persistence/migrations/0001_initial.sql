CREATE TABLE worlds (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  seed NUMERIC(20,0) NOT NULL CHECK (seed > 0),
  tick_duration_seconds INTEGER NOT NULL CHECK (tick_duration_seconds > 0),
  run_state TEXT NOT NULL CHECK (run_state IN ('created','running','paused')),
  tick BIGINT NOT NULL DEFAULT 0,
  state_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE simulation_config (
  world_id UUID PRIMARY KEY REFERENCES worlds(id) ON DELETE CASCADE,
  config JSONB NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE world_checkpoints (
  id BIGSERIAL PRIMARY KEY,
  world_id UUID NOT NULL REFERENCES worlds(id) ON DELETE CASCADE,
  tick BIGINT NOT NULL,
  state_hash TEXT NOT NULL,
  snapshot JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (tick >= 0)
);
CREATE TABLE command_events (
  id BIGSERIAL PRIMARY KEY,
  world_id UUID NOT NULL REFERENCES worlds(id) ON DELETE CASCADE,
  command TEXT NOT NULL,
  world_tick BIGINT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX command_events_world_idx ON command_events(world_id, id);
