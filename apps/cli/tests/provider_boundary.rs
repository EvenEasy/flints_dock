//! A non-Jupiter provider exercises the application contract without HTTP DTOs.
use dock_flints::app::swap::*;
use solana_keypair::Keypair;
use solana_signer::Signer;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
struct AlternateProvider(AtomicUsize);
impl SwapProvider for AlternateProvider {
    async fn build_swap(&self, request: &SwapRequest) -> Result<PreparedSwap> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(PreparedSwap {
            request: request.clone(),
            quote: SwapQuote {
                input_mint: request.mint.to_string(),
                output_mint: dock_flints::core::WRAPPED_SOL.into(),
                raw_amount: request.raw_amount.to_string(),
                expected_out_lamports: 1000,
                min_out_lamports: 995,
                expected_out_sol: "0.000001".into(),
                min_out_sol: "0.000000995".into(),
                price_impact_pct: 0.1,
                slippage_bps: request.slippage_bps,
                route_exists: true,
                route_labels: vec!["alternate".into()],
            },
            build: SwapTransaction {
                compute_budget_instructions: vec![],
                setup_instructions: vec![],
                swap_instruction: solana_instruction::Instruction {
                    program_id: request.mint,
                    accounts: vec![],
                    data: vec![],
                },
                cleanup_instruction: None,
                other_instructions: vec![],
                tip_instruction: None,
                lookup_tables: vec![],
                blockhash: Default::default(),
                last_valid_block_height: 100,
            },
            requested_at: Instant::now(),
        })
    }
}
struct FakeChain;
impl SwapExecutor for FakeChain {
    async fn check_input(&self, _: &SwapRequest) -> Result<()> {
        Ok(())
    }
    async fn submit(&self, fresh: PreparedSwap, _: &Keypair, _: &SwapLimits) -> Result<String> {
        assert_eq!(fresh.quote.route_labels, ["alternate"]);
        Ok("mock-confirmed".into())
    }
}
#[tokio::test]
async fn alternate_provider_can_preview_and_execute_without_jupiter_types() {
    let signer = Keypair::new();
    let request = SwapRequest {
        wallet: signer.pubkey(),
        mint: solana_pubkey::Pubkey::new_from_array([7; 32]),
        raw_amount: 42,
        slippage_bps: 50,
    };
    let provider = AlternateProvider(AtomicUsize::new(0));
    let limits = SwapLimits::default();
    let approved = preview(&provider, &request, &limits).await.unwrap();
    let receipt = execute(&provider, &FakeChain, &approved, &signer, &limits)
        .await
        .unwrap();
    assert_eq!(receipt.signature, "mock-confirmed");
    assert_eq!(provider.0.load(Ordering::SeqCst), 2); // Execution rebuilds through the same boundary.
}
