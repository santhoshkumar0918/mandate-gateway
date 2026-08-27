use crate::error::ReconciliationError;

/// A captured refund result returned by the provider.
pub struct IssuedRefund {
    pub id: String,
    pub status: String,
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
