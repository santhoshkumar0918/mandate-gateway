-- Verify-then-pay recovery is money-aware: an order that was created but
-- never fulfilled AND never charged is *released* (terminal, no refund)
-- rather than refunded, because there is no money to return. Allow that
-- terminal status in the mismatch lifecycle check constraint.
ALTER TABLE mismatches
    DROP CONSTRAINT mismatches_status_check;

ALTER TABLE mismatches
    ADD CONSTRAINT mismatches_status_check
    CHECK (status IN ('detected', 'refund_initiated', 'refund_completed', 'released'));
