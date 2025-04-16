-- Your SQL goes here
ALTER TABLE rewards ADD CONSTRAINT unique_miner_pool UNIQUE (miner_id, pool_id);
