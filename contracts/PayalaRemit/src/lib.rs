#![no_std]

//! PayalaRemit — on-chain remittance ledger and instant settlement contract.
//!
//! Rosa (sender, Dubai) calls `send_remittance` to push USDC to her mother's
//! wallet (recipient, Cebu) in a single atomic Soroban call. The contract
//! moves the token balance AND writes a permanent, queryable remittance
//! record in the same transaction — this is the proof that the "off-ramp"
//! (local anchor cash-out) can be triggered deterministically once funds
//! have actually arrived on-chain, instead of trusting a remittance counter's
//! internal ledger.
//!
//! NOTE: The AED -> USDC conversion itself happens via Stellar's classic
//! path-payment / built-in DEX operation (outside Soroban). This contract
//! represents the USDC-leg settlement + anchor cash-out confirmation layer,
//! which is the part that benefits from programmable, auditable logic.

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec,
};

/// Status of a remittance through its lifecycle.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemittanceStatus {
    /// USDC has landed in the recipient's wallet on-chain.
    Settled,
    /// Recipient has cashed out to PHP via the local anchor.
    CashedOut,
}

/// A single remittance record, written at the moment of transfer.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Remittance {
    pub id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
    pub status: RemittanceStatus,
    pub created_at: u64,
}

/// Storage keys. Mirrors the instance/persistent split used in
/// soroban-community-treasury: counters + config live in Instance storage,
/// individual records live in Persistent storage.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Initialization guard — prevents re-initializing the admin/anchor config.
    Init,
    /// The address authorized to confirm anchor cash-outs (the PH anchor).
    Anchor,
    /// Monotonically increasing remittance ID counter.
    NextId,
    /// Individual remittance record, keyed by ID.
    RemittanceRec(u64),
    /// All remittance IDs belonging to a given recipient (for "my history" UI).
    RecipientIndex(Address),
}

const REMITTANCE_KEY: Symbol = Symbol::short("rem");

#[contract]
pub struct PayalaRemitContract;

#[contractimpl]
impl PayalaRemitContract {
    /// Set up the contract once. `anchor` is the address allowed to confirm
    /// PHP cash-outs on behalf of the recipient (simulating the local anchor
    /// in Cebu that hands over physical/mobile-money pesos).
    pub fn initialize(env: Env, anchor: Address) {
        if env.storage().instance().has(&DataKey::Init) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Init, &true);
        env.storage().instance().set(&DataKey::Anchor, &anchor);
        env.storage().instance().set(&DataKey::NextId, &0u64);
    }

    /// CORE MVP FUNCTION.
    /// Rosa (sender) pushes `amount` of `token` (USDC) directly to her
    /// mother (recipient). This performs the real token transfer AND
    /// writes the on-chain record in one atomic call — there is no
    /// intermediate "pending" state because Stellar settlement is final
    /// in ~5 seconds, unlike a remittance counter's multi-day clearing.
    pub fn send_remittance(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        amount: i128,
    ) -> u64 {
        sender.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Move the actual USDC from sender to recipient via the token contract.
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &recipient, &amount);

        // Allocate the next remittance ID.
        let mut next_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0);
        let id = next_id;
        next_id += 1;
        env.storage().instance().set(&DataKey::NextId, &next_id);

        let record = Remittance {
            id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token,
            amount,
            status: RemittanceStatus::Settled,
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::RemittanceRec(id), &record);

        // Append to recipient's history index so her app can list past
        // remittances without scanning the whole ledger.
        let mut history: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::RecipientIndex(recipient.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::RecipientIndex(recipient), &history);

        env.events()
            .publish((REMITTANCE_KEY, Symbol::new(&env, "settled")), id);

        id
    }

    /// The local PH anchor confirms that the recipient has cashed USDC out
    /// to physical/mobile-money PHP. Only the configured anchor may call
    /// this — it's the on-chain proof that the last mile (off-ramp)
    /// actually completed, which a remittance counter cannot give you.
    pub fn confirm_cash_out(env: Env, anchor: Address, remittance_id: u64) {
        anchor.require_auth();

        let configured_anchor: Address = env
            .storage()
            .instance()
            .get(&DataKey::Anchor)
            .expect("contract not initialized");
        if anchor != configured_anchor {
            panic!("caller is not the authorized anchor");
        }

        let mut record: Remittance = env
            .storage()
            .persistent()
            .get(&DataKey::RemittanceRec(remittance_id))
            .expect("remittance not found");

        if record.status == RemittanceStatus::CashedOut {
            panic!("remittance already cashed out");
        }

        record.status = RemittanceStatus::CashedOut;
        env.storage()
            .persistent()
            .set(&DataKey::RemittanceRec(remittance_id), &record);

        env.events()
            .publish((REMITTANCE_KEY, Symbol::new(&env, "cashed_out")), remittance_id);
    }

    /// Fetch a single remittance record — used by the UI to show
    /// Rosa's mother's updated status instantly during the demo.
    pub fn get_remittance(env: Env, remittance_id: u64) -> Remittance {
        env.storage()
            .persistent()
            .get(&DataKey::RemittanceRec(remittance_id))
            .expect("remittance not found")
    }

    /// List all remittance IDs ever received by this recipient.
    pub fn get_history(env: Env, recipient: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::RecipientIndex(recipient))
            .unwrap_or_else(|| Vec::new(&env))
    }
}