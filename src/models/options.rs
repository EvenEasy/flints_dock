#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSelection {
    pub balance: bool,
    pub tokens: bool,
    pub all_tokens: bool,
    pub nfts: bool,
    pub cnfts: bool,
}

impl ScanSelection {
    pub const ALL: Self = Self {
        balance: true,
        tokens: true,
        // The raw view overlaps the semantic categories and is explicitly opt-in.
        all_tokens: false,
        nfts: true,
        cnfts: true,
    };

    pub fn is_empty(self) -> bool {
        !self.balance && !self.tokens && !self.all_tokens && !self.nfts && !self.cnfts
    }

    pub fn needs_token_accounts(self) -> bool {
        self.tokens || self.all_tokens || self.nfts
    }
    pub fn needs_prices(self) -> bool {
        self.balance || self.tokens || self.all_tokens
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub selection: ScanSelection,
    pub no_prices: bool,
    pub verbose: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            selection: ScanSelection::ALL,
            no_prices: false,
            verbose: false,
        }
    }
}
