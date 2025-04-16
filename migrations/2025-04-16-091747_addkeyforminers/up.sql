-- Your SQL goes here
ALTER TABLE miners
ADD CONSTRAINT unique_pubkey UNIQUE (pubkey);
