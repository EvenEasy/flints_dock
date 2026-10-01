use crate::core::PriceReport;

// Static dispatch keeps the single optional provider simple, while allowing
// deterministic providers in consumers/tests without an object-safe async layer.
pub trait PriceProvider {
    fn get_prices(&self, mints: &[String])
    -> impl std::future::Future<Output = PriceReport> + Send;
}

use crate::core::asset::WRAPPED_SOL;
use crate::{core::amount::approximate_value, core::*};

pub async fn price_portfolio<P: PriceProvider>(
    portfolio: &mut WalletSnapshot,
    no_prices: bool,
    provider: Option<&P>,
) -> ScanStatus {
    if no_prices {
        return ScanStatus::Skipped("Disabled by --no-prices".into());
    }
    let special_mints: std::collections::BTreeSet<_> = portfolio
        .mints
        .iter()
        .filter(|info| {
            info.extension_types
                .iter()
                .any(|kind| matches!(kind.as_str(), "ScaledUiAmount" | "InterestBearingConfig"))
        })
        .map(|info| info.mint.as_str())
        .collect();
    // Only quote balances that can actually be valued in the selected categories.
    let mut mints: Vec<_> = portfolio
        .tokens
        .iter()
        .chain(&portfolio.all_tokens)
        .filter(|token| {
            token.kind.is_fungible()
                && token.total_raw_amount > 0
                && !special_mints.contains(token.mint.as_str())
        })
        .map(|token| token.mint.clone())
        .collect();
    if portfolio.native_sol.is_some() {
        mints.push(WRAPPED_SOL.into());
    }
    mints.sort();
    mints.dedup();
    if mints.is_empty() {
        return ScanStatus::Skipped("No priceable assets in selected categories".into());
    }
    let Some(provider) = provider else {
        return ScanStatus::Skipped("No price provider configured".into());
    };
    let mut report = provider.get_prices(&mints).await;
    if let Some(native) = &mut portfolio.native_sol
        && let Some(quote) = report
            .quotes
            .get(WRAPPED_SOL)
            .filter(|quote| quote.decimals == 9)
    {
        native.price = Some(quote.clone());
        native.value_usd = approximate_value(native.lamports.into(), 9, quote.usd);
    }
    for token in portfolio.tokens.iter_mut().chain(&mut portfolio.all_tokens) {
        // A provider must not accidentally value excluded units or NFT mints.
        if mints.binary_search(&token.mint).is_err() || !token.kind.is_fungible() {
            continue;
        }
        if let Some(quote) = report.quotes.get(&token.mint) {
            if token.decimals == Some(quote.decimals) {
                token.value_usd =
                    approximate_value(token.total_raw_amount, quote.decimals, quote.usd);
                token.price = Some(quote.clone());
            } else {
                report.status = ScanStatus::Partial(
                    "Some price decimals do not match on-chain mint decimals".into(),
                );
            }
        }
    }
    report.status
}

// The unpriced use case can select a concrete provider type without constructing
// an HTTP client. The None branch prevents this method from being called.
impl PriceProvider for () {
    async fn get_prices(&self, _: &[String]) -> PriceReport {
        PriceReport {
            status: ScanStatus::Skipped("No price provider".into()),
            quotes: Default::default(),
        }
    }
}
