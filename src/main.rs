use tokio;
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use flints_station::scanner;

#[derive(Parser)]
struct Cli {
    /// Account pubkey
    #[arg(short, long)]
    pubkey: Pubkey,

    /// Account secret seed
    //#[arg(short, long)]
    //seed: String,

    /// RPC url; default solana mainnet rpc url
    #[arg(short, long, default_value="https://api.mainnet.solana.com")]
    rpc_url: String,

}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let wallet = &args.pubkey;
    let client = &RpcClient::new(args.rpc_url);

    let mut total_amount_sol: f64 = 0.0;

    // Fetch SOL balance
    let balance_lam = scanner::sol::get_solana_balance(&client, &wallet).await.unwrap();
    let balance = balance_lam as f64/LAMPORTS_PER_SOL as f64;

    total_amount_sol+=balance;

    // Display result
    println!(
        "Balance: {:.?} SOL ({:?} lamports)",
        &balance,
        &balance_lam
    );

    let accounts = scanner::tokens::get_account_tokens(
        &client,
        &wallet,
    ).await.unwrap();

    let sol_price = 117.39;

    for token in &accounts {
        let raw_amount = token.amount.parse::<u64>().unwrap();

        if raw_amount == 0 {
            continue;
        }

        println!(
            "mint={} raw={} decimals={} ui={} account={}",
            token.mint,
            raw_amount,
            token.decimals,
            raw_amount as f64 / 10_f64.powi(token.decimals as i32),
            token.address,
        );

        if raw_amount != 1 || token.decimals != 0 {
            continue;
        }

        // Решта NFT-сканера...
    }

    let nfts = scanner::core::get_core_nfts(&client, &wallet).await.unwrap();

    println!("Found {} Core NFTs", nfts.len());

    for nft in nfts {
        println!("{:#?}", nft);
    }




    println!(
        "
        Token count: {};
        Total amount: {} SOL;
        Amount: {}",
        &accounts.iter().count(),
        total_amount_sol,
        total_amount_sol * sol_price,
    );
}
