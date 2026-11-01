-- Vendor service tier of a request (e.g. OpenAI Batch, Flex, Fast), only
-- when the source records it. NULL means unknown and bills as Standard.
ALTER TABLE events ADD COLUMN service_tier TEXT
    CHECK (service_tier IS NULL OR service_tier IN ('standard', 'batch', 'flex', 'fast'));
