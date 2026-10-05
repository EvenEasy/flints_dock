//! Preview and execute one exact-input swap through replaceable boundaries.
pub use crate::core::{error::*, swap::*};
use solana_signer::Signer;

/// Define the replaceable route-building boundary without exposing provider wire responses.
pub trait SwapProvider {
    /// Return a fresh quote and native Solana transaction ingredients for the exact request.
    /// Implementations must validate response identity and map provider failures to semantic
    /// errors.
    fn build_swap(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<PreparedSwap>> + Send;
}

/// Separate holding checks and signed transaction submission from route selection.
pub trait SwapExecutor {
    /// Verify the requested holdings and supported input semantics before execution.
    fn check_input(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Return a confirmed signature after submitting the fresh route with the local signer.
    /// Ambiguous submission must preserve the transaction signature in the returned error.
    fn submit(
        &self,
        fresh: PreparedSwap,
        signer: &solana_keypair::Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// Return a wallet-bound quote after validating the request, age and price impact.
/// Does not accept a signer or submit transactions; provider and validation errors propagate.
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

/// Return a confirmed swap receipt using a newly built route for the approved request.
/// Rejects a different signer, stale quote, excessive impact or a worse approved minimum.
/// An uncertain submission is returned with its signature and is never rebuilt automatically.
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

    // Revalidate holdings before requesting the transaction that will actually be submitted.
    executor.check_input(&approved.request).await?;
    let fresh = provider.build_swap(&approved.request).await?;
    fresh.ensure_fresh(limits)?;
    fresh.quote.check_impact(limits)?;

    // Stop if current market conditions violate the previously approved minimum.
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
