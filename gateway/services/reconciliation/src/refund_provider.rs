use crate::error::ReconciliationError;

/// A captured refund result returned by the provider.
pub struct IssuedRefund {
    pub id: String,
    pub status: String,
}

/// A payment id is "self-captured" when it was recorded in our ledger at order
/// creation with no Razorpay checkout behind it (test-mode flow). Such
/// payments carry a `pay_self_` prefix.
pub fn is_self_captured(payment_id: &str) -> bool {
    payment_id.starts_with("pay_self_")
}

/// Boundary for issuing refunds, so the reconciliation service can be tested
/// without hitting Razorpay.
///
/// The real implementation calls [`razorpay_client::RazorpayClient`]; tests
/// provide an in-memory fake.
pub trait RefundProvider: Send + Sync {
    fn issue_refund(
        &self,
        payment_id: &str,
        amount: Option<i64>,
    ) -> impl std::future::Future<Output = Result<IssuedRefund, ReconciliationError>> + Send;
}

/// Default provider backed by the real Razorpay client.
#[derive(Clone)]
pub struct RazorpayRefundProvider(pub razorpay_client::RazorpayClient);

impl RefundProvider for RazorpayRefundProvider {
    async fn issue_refund(
        &self,
        payment_id: &str,
        amount: Option<i64>,
    ) -> Result<IssuedRefund, ReconciliationError> {
        // Self-captured test payments (recorded in our ledger when no Razorpay
        // checkout exists) have no real `pay_...` counterpart in the sandbox, so
        // the external refund call would always be rejected. For these the refund
        // is recorded in our ledger directly (Option A): the recovery is real at
        // our layer, the gateway round-trip is best-effort. Real `pay_...` ids
        // still go through the sandbox unchanged.
        if is_self_captured(payment_id) {
            return Ok(IssuedRefund {
                id: format!("refund_self_{payment_id}"),
                status: "processed".to_string(),
            });
        }
        let refund = self
            .0
            .create_refund(payment_id, amount)
            .await
            .map_err(|e| ReconciliationError::RefundFailed(e.to_string()))?;
        Ok(IssuedRefund {
            id: refund.id,
            status: refund.status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::is_self_captured;

    #[test]
    fn self_captured_payments_are_tagged() {
        assert!(is_self_captured("pay_self_order_TXDh5"));
    }

    #[test]
    fn real_payments_are_not_self_captured() {
        assert!(!is_self_captured("pay_OkMqLfV3w5uiH7"));
        assert!(!is_self_captured(""));
    }
}
