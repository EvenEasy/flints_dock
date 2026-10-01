//! Preview and execute one exact-input swap through replaceable boundaries.
pub use crate::core::{error::*, swap::*};
use solana_signer::Signer;
pub trait SwapProvider {
    fn build_swap(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<PreparedSwap>> + Send;
}
pub trait SwapExecutor {
    fn check_input(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn submit(
        &self,
        fresh: PreparedSwap,
        signer: &solana_keypair::Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

pub async fn preview(
    provider: &impl SwapProvider,
    request: &SwapRequest,
    limits: &SwapLimits,
) -> Result<SwapPreview> {
    request.validate()?;
    let built = provider.build_swap(request).await?;
    built.ensure_fresh(limits)?;
    built.quote.check_impact(limits)?;
    Ok(SwapPreview {
        request: request.clone(),
        quote: built.quote,
    })
}

/// Never accepts preview instructions: every execution builds a new route/transaction.
/// There is no retry that could rebuild and submit a second economic transaction.
pub async fn execute(
    provider: &impl SwapProvider,
    executor: &impl SwapExecutor,
    approved: &SwapPreview,
    signer: &solana_keypair::Keypair,
    limits: &SwapLimits,
) -> Result<SwapReceipt> {
    if signer.pubkey() != approved.request.wallet {
        return Err(SwapError::InvalidRequest(
            "keypair does not match preview wallet".into(),
        ));
    }
    executor.check_input(&approved.request).await?;
    let fresh = provider.build_swap(&approved.request).await?;
    fresh.ensure_fresh(limits)?;
    fresh.quote.check_impact(limits)?;
    if fresh.quote.min_out_lamports < approved.quote.min_out_lamports {
        return Err(SwapError::PreviewChanged);
    }
    let quote = fresh.quote.clone();
    let signature = executor.submit(fresh, signer, limits).await?;
    Ok(SwapReceipt {
        status: "confirmed",
        signature,
        output_asset: "native SOL",
        fresh_quote: quote,
    })
}
