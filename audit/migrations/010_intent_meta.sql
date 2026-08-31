-- Surface the buyer agent's parsed intent (LLM reasoning / selection mode)
-- and link each intent to the order it produced, so per-order views can show
-- why the agent picked a product and whether it was later fulfilled.
ALTER TABLE intents ADD COLUMN IF NOT EXISTS reasoning TEXT;
ALTER TABLE intents ADD COLUMN IF NOT EXISTS selection_method TEXT;
ALTER TABLE intents ADD COLUMN IF NOT EXISTS order_id TEXT;

CREATE INDEX IF NOT EXISTS idx_intents_order_id ON intents (order_id);
