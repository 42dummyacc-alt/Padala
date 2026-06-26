#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

/// Deploys a Stellar Asset Contract to stand in for USDC in tests.
/// Returns (token contract address, admin client, token client).
fn setup_token<'a>(env: &Env) -> (Address, StellarAssetClient<'a>, TokenClient<'a>) {
    let token_admin = Address::generate(env);
    let contract_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let asset_client = StellarAssetClient::new(env, &contract_address.address());
    let token_client = TokenClient::new(env, &contract_address.address());
    (contract_address.address(), asset_client, token_client)
}

fn setup<'a>() -> (
    Env,
    PayalaRemitContractClient<'a>,
    Address, // anchor
    Address, // sender (Rosa)
    Address, // recipient (mother)
    Address, // usdc token
    StellarAssetClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PayalaRemitContract);
    let client = PayalaRemitContractClient::new(&env, &contract_id);

    let anchor = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, asset_admin, _token_client) = setup_token(&env);
    // Mint Rosa 1000 USDC (units) so she has funds to send.
    asset_admin.mint(&sender, &1000_0000000i128);

    client.initialize(&anchor);

    (env, client, anchor, sender, recipient, token_addr, asset_admin)
}

#[test]
fn test_happy_path_send_and_cash_out() {
    // Test 1 (Happy path): Rosa sends USDC end-to-end and the anchor
    // confirms cash-out — this is the exact MVP demo flow.
    let (_env, client, anchor, sender, recipient, token_addr, _asset_admin) = setup();

    let id = client.send_remittance(&sender, &recipient, &token_addr, &150_0000000i128);
    let record = client.get_remittance(&id);
    assert_eq!(record.status, RemittanceStatus::Settled);
    assert_eq!(record.amount, 150_0000000i128);

    client.confirm_cash_out(&anchor, &id);
    let updated = client.get_remittance(&id);
    assert_eq!(updated.status, RemittanceStatus::CashedOut);
}

#[test]
#[should_panic(expected = "caller is not the authorized anchor")]
fn test_unauthorized_anchor_cannot_confirm_cash_out() {
    // Test 2 (Edge case): a random address (not the configured anchor)
    // must not be able to mark a remittance as cashed out.
    let (env, client, _anchor, sender, recipient, token_addr, _asset_admin) = setup();

    let id = client.send_remittance(&sender, &recipient, &token_addr, &50_0000000i128);

    let impostor = Address::generate(&env);
    client.confirm_cash_out(&impostor, &id);
}

#[test]
fn test_storage_reflects_correct_state_after_send() {
    // Test 3 (State verification): after send_remittance, storage must
    // show the correct token balances AND the recipient's history index.
    let (_env, client, _anchor, sender, recipient, token_addr, _asset_admin) = setup();

    let token_client = token::Client::new(&_env, &token_addr);
    let amount = 300_0000000i128;

    let id = client.send_remittance(&sender, &recipient, &token_addr, &amount);

    // Balance moved on-chain.
    assert_eq!(token_client.balance(&recipient), amount);
    assert_eq!(token_client.balance(&sender), 1000_0000000i128 - amount);

    // History index updated for the recipient.
    let history = client.get_history(&recipient);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap(), id);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_zero_amount_remittance_rejected() {
    // Test 4 (Edge case): zero/negative amounts must be rejected so the
    // contract never records a meaningless or exploitable transfer.
    let (_env, client, _anchor, sender, recipient, token_addr, _asset_admin) = setup();

    client.send_remittance(&sender, &recipient, &token_addr, &0i128);
}

#[test]
#[should_panic(expected = "remittance already cashed out")]
fn test_double_cash_out_rejected() {
    // Test 5 (Edge case): the anchor cannot confirm the same remittance
    // as cashed out twice (would otherwise allow double-counting payouts
    // in the anchor's own reconciliation).
    let (_env, client, anchor, sender, recipient, token_addr, _asset_admin) = setup();

    let id = client.send_remittance(&sender, &recipient, &token_addr, &75_0000000i128);
    client.confirm_cash_out(&anchor, &id);
    client.confirm_cash_out(&anchor, &id);
}
