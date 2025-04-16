-- This file should undo anything in `up.sql`
ALTER TABLE rewards DROP CONSTRAINT unique_miner_pool;
