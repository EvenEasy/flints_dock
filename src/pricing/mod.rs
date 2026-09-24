pub mod jupiter;
use crate::models::PriceReport;

// Static dispatch keeps the single optional provider simple, while allowing
// deterministic providers in consumers/tests without an object-safe async layer.
pub trait PriceProvider {
    fn get_prices(&self, mints: &[String])
    -> impl std::future::Future<Output = PriceReport> + Send;
}
