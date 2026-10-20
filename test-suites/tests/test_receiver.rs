use soroban_sdk::{
    testutils::Address as _,
    token, vec,
    xdr::{ScErrorCode, ScErrorType},
    Address, Error, IntoVal, Vec,
};
use test_suites::test_fixture::{TestFixture, BACKSTOP_DEPOSIT, DAY, SCALAR, SWAP_DELAY};

const AUTH_ERROR: Error =
    Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction);

#[test]
fn v2_to_receiver_to_v21() {
    let f = TestFixture::create();
    let treasury = Address::generate(&f.env);
    let old_lp = f.v2.lp.balance(&f.admin);
    assert!(old_lp > BACKSTOP_DEPOSIT);
    assert!(f.v2.factory.is_pool(&f.v2.pool.address));
    assert!(f.v21.factory.is_pool(&f.v21.pool.address));
    assert_eq!(f.receiver.get_owner(), Some(f.admin.clone()));
    assert_eq!(f.receiver.decimals(), 7);
    assert_eq!(f.receiver.total_supply(), 0);

    f.owner_call("mint", (&f.v21.backstop.address, SCALAR).into_val(&f.env));
    assert_eq!(
        f.receiver.try_mint(&f.v21.backstop.address, &SCALAR),
        Err(Ok(Error::from_contract_error(900)))
    );
    f.owner_call("refund", Vec::new(&f.env));
    assert_eq!(
        f.receiver.try_refund(),
        Err(Ok(Error::from_contract_error(900)))
    );

    f.jump(DAY);
    assert_eq!(f.emitter.distribute(), DAY as i128 * SCALAR);
    assert_eq!(f.v2.backstop.distribute(), DAY as i128 * SCALAR);
    f.authorize_acquisition();
    f.receiver.acquire_backstop();
    assert_eq!(f.v2.lp.balance(&f.admin), 0);
    assert_eq!(f.v2.lp.balance(&f.receiver.address), old_lp);
    let queued = f.emitter.get_queued_swap().unwrap();
    assert_eq!(queued.new_backstop, f.receiver.address);
    assert_eq!(queued.new_backstop_token, f.receiver.address);
    assert_eq!(f.emitter.get_backstop(), f.v2.backstop.address);
    f.owner_call("refund", Vec::new(&f.env));
    assert_eq!(
        f.receiver.try_refund(),
        Err(Ok(Error::from_contract_error(900)))
    );
    assert_eq!(f.v2.lp.balance(&f.receiver.address), old_lp);
    f.jump(SWAP_DELAY - 1);
    assert_eq!(
        f.emitter.try_swap_backstop(),
        Err(Ok(Error::from_contract_error(1104)))
    );
    f.jump(1);
    f.emitter.swap_backstop();
    assert_eq!(f.emitter.get_backstop(), f.receiver.address);
    let final_v2_blnd = f.blnd.balance(&f.v2.backstop.address);
    assert_eq!(final_v2_blnd, (DAY + SWAP_DELAY) as i128 * SCALAR);

    // SC receives the old LP intact and burns it directly in a separate transaction.
    let supply = f.v2.lp.get_total_supply();
    let pool_blnd = f.blnd.balance(&f.v2.lp.address);
    let pool_usdc = f.usdc.balance(&f.v2.lp.address);
    f.owner_call("refund", Vec::new(&f.env));
    f.receiver.refund();
    assert_eq!(f.v2.lp.balance(&f.admin), old_lp);
    assert_eq!(f.v2.lp.get_total_supply(), supply);
    f.authorize(
        &f.admin,
        &f.v2.lp.address,
        "burn",
        (&f.admin, old_lp).into_val(&f.env),
    );
    token::Client::new(&f.env, &f.v2.lp.address).burn(&f.admin, &old_lp);
    assert_eq!(f.v2.lp.balance(&f.admin), 0);
    assert_eq!(f.v2.lp.get_total_supply(), supply - old_lp);
    assert_eq!(f.blnd.balance(&f.v2.lp.address), pool_blnd);
    assert_eq!(f.usdc.balance(&f.v2.lp.address), pool_usdc);

    f.owner_call("drop", (&treasury,).into_val(&f.env));
    f.receiver.drop(&treasury);
    assert_eq!(f.blnd.balance(&treasury), 50_000_000 * SCALAR);
    f.owner_call("drop", (&treasury,).into_val(&f.env));
    assert_eq!(
        f.receiver.try_drop(&treasury),
        Err(Ok(Error::from_contract_error(1101)))
    );
    f.jump(DAY);
    assert_eq!(f.emitter.distribute(), DAY as i128 * SCALAR);
    f.owner_call(
        "forward_emissions",
        (&f.v21.backstop.address,).into_val(&f.env),
    );
    f.receiver.forward_emissions(&f.v21.backstop.address);
    assert_eq!(f.blnd.balance(&f.receiver.address), 0);
    assert_eq!(
        f.blnd.balance(&f.v21.backstop.address),
        DAY as i128 * SCALAR
    );

    // The emitter compares receiver-token balances for this handover, then adopts the fresh LP.
    assert_eq!(
        f.emitter
            .try_queue_swap_backstop(&f.v21.backstop.address, &f.v21.lp.address),
        Err(Ok(Error::from_contract_error(1100)))
    );
    let handover = vec![
        &f.env,
        f.call(
            &f.receiver.address,
            "mint",
            (&f.v21.backstop.address, SCALAR).into_val(&f.env),
        ),
        f.call(
            &f.emitter.address,
            "queue_swap_backstop",
            (&f.v21.backstop.address, &f.v21.lp.address).into_val(&f.env),
        ),
    ];
    f.authorize_batch(&handover, &[(&f.v21.backstop.address, SCALAR)]);
    f.router.exec(&f.admin, &handover);
    assert_eq!(f.receiver.balance(&f.v21.backstop.address), SCALAR);
    let queued = f.emitter.get_queued_swap().unwrap();
    assert_eq!(queued.new_backstop, f.v21.backstop.address);
    assert_eq!(queued.new_backstop_token, f.v21.lp.address);
    f.jump(SWAP_DELAY);
    f.emitter.swap_backstop();
    assert_eq!(f.emitter.get_backstop(), f.v21.backstop.address);
    assert_eq!(
        f.blnd.balance(&f.receiver.address),
        SWAP_DELAY as i128 * SCALAR
    );
    f.owner_call(
        "forward_emissions",
        (&f.v21.backstop.address,).into_val(&f.env),
    );
    f.receiver.forward_emissions(&f.v21.backstop.address);
    assert_eq!(f.blnd.balance(&f.receiver.address), 0);
    assert_eq!(f.blnd.balance(&f.v2.backstop.address), final_v2_blnd);
    f.owner_call("mint", (&f.v21.backstop.address, SCALAR).into_val(&f.env));
    assert_eq!(
        f.receiver.try_mint(&f.v21.backstop.address, &SCALAR),
        Err(Ok(Error::from_contract_error(900)))
    );
    f.owner_call("refund", Vec::new(&f.env));
    assert_eq!(
        f.receiver.try_refund(),
        Err(Ok(Error::from_contract_error(900)))
    );

    // v2.1 switches from backfill accounting to real emissions and distributes normally.
    assert_eq!(f.v21.backstop.distribute(), 0);
    let before = f.blnd.balance(&f.v21.backstop.address);
    f.jump(DAY);
    assert_eq!(f.emitter.distribute(), DAY as i128 * SCALAR);
    assert_eq!(f.v21.backstop.distribute(), DAY as i128 * SCALAR);
    assert_eq!(
        f.blnd.balance(&f.v21.backstop.address),
        before + DAY as i128 * SCALAR
    );
    f.owner_call("renounce_ownership", Vec::new(&f.env));
    f.receiver.renounce_ownership();
    assert_eq!(f.receiver.get_owner(), None);
}

#[test]
fn receiver_actions_require_owner_authorization() {
    let f = TestFixture::create();
    let stranger = Address::generate(&f.env);
    f.authorize(
        &stranger,
        &f.receiver.address,
        "acquire_backstop",
        Vec::new(&f.env),
    );
    assert_eq!(f.receiver.try_acquire_backstop(), Err(Ok(AUTH_ERROR)));
    f.authorize(
        &stranger,
        &f.receiver.address,
        "mint",
        (&stranger, SCALAR).into_val(&f.env),
    );
    assert_eq!(f.receiver.try_mint(&stranger, &SCALAR), Err(Ok(AUTH_ERROR)));
    f.authorize(
        &stranger,
        &f.receiver.address,
        "forward_emissions",
        (&stranger,).into_val(&f.env),
    );
    assert_eq!(
        f.receiver.try_forward_emissions(&stranger),
        Err(Ok(AUTH_ERROR))
    );
    f.authorize(&stranger, &f.receiver.address, "refund", Vec::new(&f.env));
    assert_eq!(f.receiver.try_refund(), Err(Ok(AUTH_ERROR)));
    f.authorize(
        &stranger,
        &f.receiver.address,
        "drop",
        (&stranger,).into_val(&f.env),
    );
    assert_eq!(f.receiver.try_drop(&stranger), Err(Ok(AUTH_ERROR)));
    assert_eq!(f.v2.lp.balance(&f.receiver.address), 0);
    assert_eq!(f.receiver.total_supply(), 0);
}

#[test]
fn rejected_acquisition_preserves_lp_and_existing_swap() {
    let f = TestFixture::create();
    let before = f.v2.lp.balance(&f.admin);
    f.emitter.queue_swap_backstop(&f.admin, &f.v2.lp.address);
    let queued = f.emitter.get_queued_swap().unwrap();
    f.authorize_acquisition();
    assert_eq!(
        f.receiver.try_acquire_backstop(),
        Err(Ok(Error::from_contract_error(1103)))
    );
    assert_eq!(f.v2.lp.balance(&f.admin), before);
    assert_eq!(f.v2.lp.balance(&f.receiver.address), 0);
    assert_eq!(f.emitter.get_queued_swap().unwrap(), queued);
}

#[test]
fn owner_can_choose_forwarding_destination_and_renounce() {
    let f = TestFixture::create();
    f.activate_receiver();
    f.jump(DAY);
    f.emitter.distribute();
    let destination = Address::generate(&f.env);
    f.owner_call(
        "forward_emissions",
        (&f.v21.backstop.address,).into_val(&f.env),
    );
    assert_eq!(
        f.receiver.try_forward_emissions(&destination),
        Err(Ok(AUTH_ERROR))
    );
    assert_eq!(f.blnd.balance(&f.receiver.address), DAY as i128 * SCALAR);
    f.owner_call("forward_emissions", (&destination,).into_val(&f.env));
    f.receiver.forward_emissions(&destination);
    assert_eq!(f.blnd.balance(&destination), DAY as i128 * SCALAR);
    assert_eq!(f.blnd.balance(&f.v21.backstop.address), 0);
    f.owner_call("forward_emissions", (&destination,).into_val(&f.env));
    f.receiver.forward_emissions(&destination);
    f.owner_call("renounce_ownership", Vec::new(&f.env));
    f.receiver.renounce_ownership();
    f.owner_call("forward_emissions", (&destination,).into_val(&f.env));
    assert_eq!(
        f.receiver.try_forward_emissions(&destination),
        Err(Ok(Error::from_contract_error(2100)))
    );
}

#[test]
fn refund_follows_accepted_ownership_transfer() {
    let f = TestFixture::create();
    f.activate_receiver();
    let amount = f.v2.lp.balance(&f.receiver.address);
    let new_owner = Address::generate(&f.env);
    let expires = f.env.ledger().sequence() + 100;
    f.owner_call("transfer_ownership", (&new_owner, expires).into_val(&f.env));
    f.receiver.transfer_ownership(&new_owner, &expires);
    f.authorize(
        &new_owner,
        &f.receiver.address,
        "accept_ownership",
        Vec::new(&f.env),
    );
    f.receiver.accept_ownership();
    f.owner_call("refund", Vec::new(&f.env));
    assert_eq!(f.receiver.try_refund(), Err(Ok(AUTH_ERROR)));
    f.authorize(&new_owner, &f.receiver.address, "refund", Vec::new(&f.env));
    f.receiver.refund();
    assert_eq!(f.v2.lp.balance(&new_owner), amount);
    assert_eq!(f.v2.lp.balance(&f.admin), 0);
    assert_eq!(f.v2.lp.balance(&f.receiver.address), 0);
}

#[test]
fn owner_mint_uses_default_amount_validation_and_supports_repeated_mints() {
    let f = TestFixture::create();
    f.activate_receiver();
    let to = &f.v21.backstop.address;
    f.owner_call("mint", (to, -1i128).into_val(&f.env));
    assert_eq!(
        f.receiver.try_mint(to, &-1),
        Err(Ok(Error::from_contract_error(103)))
    );
    for amount in [0, SCALAR, 1] {
        f.owner_call("mint", (to, amount).into_val(&f.env));
        f.receiver.mint(to, &amount);
    }
    assert_eq!(f.receiver.balance(to), SCALAR + 1);
    assert_eq!(f.receiver.total_supply(), SCALAR + 1);
}

#[test]
fn self_mint_lets_anyone_cancel_a_swap() {
    let f = TestFixture::create();
    f.activate_receiver();
    let successor = &f.v21.backstop.address;
    let receiver = &f.receiver.address;
    let handover = vec![
        &f.env,
        f.call(receiver, "mint", (successor, SCALAR).into_val(&f.env)),
        f.call(
            &f.emitter.address,
            "queue_swap_backstop",
            (successor, &f.v21.lp.address).into_val(&f.env),
        ),
    ];
    f.authorize_batch(&handover, &[(successor, SCALAR)]);
    f.router.exec(&f.admin, &handover);
    f.env.mock_auths(&[]);
    assert_eq!(
        f.emitter.try_cancel_swap_backstop(),
        Err(Ok(Error::from_contract_error(1105)))
    );
    f.owner_call("mint", (receiver, SCALAR).into_val(&f.env));
    f.receiver.mint(receiver, &SCALAR);
    assert_eq!(f.receiver.balance(receiver), SCALAR);
    f.env.mock_auths(&[]);
    f.emitter.cancel_swap_backstop();
    assert_eq!(f.emitter.get_queued_swap(), None);
    assert_eq!(
        f.emitter
            .try_queue_swap_backstop(successor, &f.v21.lp.address),
        Err(Ok(Error::from_contract_error(1100)))
    );
}

#[test]
fn router_batch_rolls_back_the_mint_when_its_queue_fails() {
    let f = TestFixture::create();
    f.activate_receiver();
    let successor = &f.v21.backstop.address;
    let batch = vec![
        &f.env,
        f.call(
            &f.receiver.address,
            "mint",
            (successor, SCALAR).into_val(&f.env),
        ),
        f.call(
            &f.emitter.address,
            "queue_swap_backstop",
            (&Address::generate(&f.env), &f.v21.lp.address).into_val(&f.env),
        ),
    ];
    f.authorize_batch(&batch, &[(successor, SCALAR)]);
    assert_eq!(
        f.router.try_exec(&f.admin, &batch).err(),
        Some(Ok(Error::from_contract_error(1100)))
    );
    assert_eq!(f.receiver.balance(successor), 0);
    assert_eq!(f.emitter.get_queued_swap(), None);
}
