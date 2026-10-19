//! Receiver and internal handover token for the Blend emitter.
#![no_std]

mod emitter {
    soroban_sdk::contractimport!(file = "wasm/emitter_v1.0.0.wasm");
}

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, token,
    unwrap::UnwrapOptimized, vec, Address, Env, MuxedAddress, String, Symbol,
};
use stellar_access::ownable::{self, Ownable};
use stellar_macros::only_owner;
use stellar_tokens::fungible::{Base, FungibleToken};

use emitter::Client as EmitterClient;

const EMITTER: Symbol = symbol_short!("Emitter");
const LP_TOKEN: Symbol = symbol_short!("LPToken");
const BLND: Symbol = symbol_short!("BLNDTkn");

const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17280;
const INSTANCE_EXTEND_AMOUNT: u32 = 31 * 17280;

/// The optional one-time emitter drop, in BLND's seven-decimal units.
pub const DROP_AMOUNT: i128 = 50_000_000 * 10_000_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReceiverError {
    NotCurrentBackstop = 900,
}

#[contract]
pub struct BlendReceiver;

#[contractimpl]
impl BlendReceiver {
    /// Sets the OZ owner and three immutable integration entries.
    pub fn __constructor(
        e: &Env,
        admin: Address,
        emitter: Address,
        v1_lp_token: Address,
        blnd: Address,
    ) {
        extend_instance(e);
        ownable::set_owner(e, &admin);
        e.storage().instance().set(&EMITTER, &emitter);
        e.storage().instance().set(&LP_TOKEN, &v1_lp_token);
        e.storage().instance().set(&BLND, &blnd);
        Base::set_metadata(
            e,
            7,
            String::from_str(e, "Blend receiver"),
            String::from_str(e, "BRECV"),
        );
    }

    /// Transfers the owner's entire old LP balance and queues this receiver.
    /// Emitter rejection rolls the LP transfer back atomically.
    #[only_owner]
    pub fn acquire_backstop(e: &Env) {
        extend_instance(e);
        let admin = ownable::get_owner(e).unwrap_optimized();
        let receiver = e.current_contract_address();
        let lp: Address = e.storage().instance().get(&LP_TOKEN).unwrap_optimized();
        let lp = token::Client::new(e, &lp);
        lp.transfer(&admin, &receiver, &lp.balance(&admin));
        let emitter: Address = e.storage().instance().get(&EMITTER).unwrap_optimized();
        EmitterClient::new(e, &emitter).queue_swap_backstop(&receiver, &receiver);
    }

    /// The owner may mint repeatedly while this receiver is emitter recipient.
    /// OZ validates the amount and updates default token balances and supply.
    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        extend_instance(e);
        check_current_backstop(e);
        Base::mint(e, &to, amount);
    }

    /// Forwards all held BLND to the owner's chosen destination.
    #[only_owner]
    pub fn forward_emissions(e: &Env, to: Address) {
        extend_instance(e);
        let receiver = e.current_contract_address();
        let blnd: Address = e.storage().instance().get(&BLND).unwrap_optimized();
        let blnd = token::Client::new(e, &blnd);
        let amount = blnd.balance(&receiver);
        if amount > 0 {
            blnd.transfer(&receiver, &to, &amount);
        }
    }

    /// Returns old LP to the current owner while this receiver is the backstop.
    #[only_owner]
    pub fn refund(e: &Env) {
        extend_instance(e);
        check_current_backstop(e);
        let receiver = e.current_contract_address();
        let lp: Address = e.storage().instance().get(&LP_TOKEN).unwrap_optimized();
        let admin = ownable::get_owner(e).unwrap_optimized();
        let lp = token::Client::new(e, &lp);
        lp.transfer(&receiver, &admin, &lp.balance(&receiver));
    }

    /// Requests the emitter's one-time 50M BLND drop for one destination.
    #[only_owner]
    pub fn drop(e: &Env, address: Address) {
        extend_instance(e);
        let emitter: Address = e.storage().instance().get(&EMITTER).unwrap_optimized();
        EmitterClient::new(e, &emitter).drop(&vec![e, (address, DROP_AMOUNT)]);
    }
}

#[contractimpl(contracttrait)]
impl Ownable for BlendReceiver {}

#[contractimpl(contracttrait)]
impl FungibleToken for BlendReceiver {
    type ContractType = Base;
}

fn check_current_backstop(e: &Env) {
    let emitter: Address = e.storage().instance().get(&EMITTER).unwrap_optimized();
    if EmitterClient::new(e, &emitter).get_backstop() != e.current_contract_address() {
        panic_with_error!(e, ReceiverError::NotCurrentBackstop);
    }
}

fn extend_instance(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
}
