use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, Ledger, LedgerInfo, MockAuth, MockAuthInvoke},
    token, vec, Address, BytesN, Env, IntoVal, String, Val, Vec,
};

use crate::{backstop, comet, emitter, pool, pool_factory, receiver};

pub const SCALAR: i128 = 10_000_000;
pub const DAY: u64 = 24 * 60 * 60;
pub const SWAP_DELAY: u64 = 31 * DAY;
pub const BACKSTOP_DEPOSIT: i128 = 30 * SCALAR;

pub struct Stack<'a> {
    pub lp: comet::Client<'a>,
    pub backstop: backstop::Client<'a>,
    pub factory: pool_factory::Client<'a>,
    pub pool: pool::Client<'a>,
}

pub struct TestFixture<'a> {
    pub env: Env,
    pub admin: Address,
    pub blnd: token::Client<'a>,
    pub usdc: token::Client<'a>,
    pub emitter: emitter::Client<'a>,
    pub receiver: receiver::Client<'a>,
    pub v2: Stack<'a>,
    pub v21: Stack<'a>,
}

impl TestFixture<'_> {
    pub fn create<'a>() -> TestFixture<'a> {
        let env = Env::new_with_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        env.cost_estimate().budget().reset_unlimited();
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            protocol_version: 25,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 180 * 17280,
            min_persistent_entry_ttl: 180 * 17280,
            max_entry_ttl: 365 * 17280,
        });
        // Bootstrap balances and contracts; each tested action uses explicit auth below.
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let blnd = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let usdc = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        token::StellarAssetClient::new(&env, &blnd).mint(&admin, &(20_000_000 * SCALAR));
        token::StellarAssetClient::new(&env, &usdc).mint(&admin, &(500_000 * SCALAR));
        let emitter = env.register(emitter::WASM, ());
        let v2 = deploy_stack(
            &env,
            &admin,
            &emitter,
            &blnd,
            &usdc,
            comet::LEGACY_WASM,
            "v2",
        );
        let v21 = deploy_stack(&env, &admin, &emitter, &blnd, &usdc, comet::WASM, "v2.1");
        let emitter = emitter::Client::new(&env, &emitter);
        emitter.initialize(&blnd, &v2.backstop.address, &v2.lp.address);
        token::StellarAssetClient::new(&env, &blnd).set_admin(&emitter.address);
        v2.backstop.distribute();
        v21.backstop.distribute();
        let receiver = env.register(
            receiver::WASM,
            (
                admin.clone(),
                emitter.address.clone(),
                v2.lp.address.clone(),
                blnd.clone(),
            ),
        );
        env.mock_auths(&[]);
        TestFixture {
            receiver: receiver::Client::new(&env, &receiver),
            blnd: token::Client::new(&env, &blnd),
            usdc: token::Client::new(&env, &usdc),
            env,
            admin,
            emitter,
            v2,
            v21,
        }
    }

    pub fn authorize(&self, signer: &Address, contract: &Address, function: &str, args: Vec<Val>) {
        self.env.mock_auths(&[MockAuth {
            address: signer,
            invoke: &MockAuthInvoke {
                contract,
                fn_name: function,
                args,
                sub_invokes: &[],
            },
        }]);
    }

    pub fn owner_call(&self, function: &str, args: Vec<Val>) {
        self.authorize(&self.admin, &self.receiver.address, function, args);
    }

    pub fn authorize_acquisition(&self) {
        let amount = self.v2.lp.balance(&self.admin);
        self.env.mock_auths(&[MockAuth {
            address: &self.admin,
            invoke: &MockAuthInvoke {
                contract: &self.receiver.address,
                fn_name: "acquire_backstop",
                args: Vec::new(&self.env),
                sub_invokes: &[MockAuthInvoke {
                    contract: &self.v2.lp.address,
                    fn_name: "transfer",
                    args: (&self.admin, &self.receiver.address, amount).into_val(&self.env),
                    sub_invokes: &[],
                }],
            },
        }]);
    }

    pub fn activate_receiver(&self) {
        self.authorize_acquisition();
        self.receiver.acquire_backstop();
        self.jump(SWAP_DELAY);
        self.env.mock_auths(&[]);
        self.emitter.swap_backstop();
    }

    pub fn jump(&self, seconds: u64) {
        self.env.ledger().with_mut(|ledger| {
            ledger.timestamp += seconds;
            ledger.sequence_number += u32::try_from(seconds.div_ceil(5)).unwrap();
        });
    }
}

fn deploy_stack<'a>(
    env: &Env,
    admin: &Address,
    emitter: &Address,
    blnd: &Address,
    usdc: &Address,
    lp_wasm: &[u8],
    name: &str,
) -> Stack<'a> {
    let lp = comet::Client::new(env, &env.register(lp_wasm, ()));
    lp.init(
        admin,
        &vec![env, blnd.clone(), usdc.clone()],
        &vec![env, 8_000_000, 2_000_000],
        &vec![env, 10_000_000 * SCALAR, 250_000 * SCALAR],
        &30_000,
    );
    let backstop = Address::generate(env);
    let factory = env.register(
        pool_factory::WASM,
        (pool_factory::PoolInitMeta {
            backstop: backstop.clone(),
            blnd_id: blnd.clone(),
            pool_hash: env.deployer().upload_contract_wasm(pool::WASM),
        },),
    );
    let factory = pool_factory::Client::new(env, &factory);
    let backstop = env.register_at(
        &backstop,
        backstop::WASM,
        (
            lp.address.clone(),
            emitter.clone(),
            blnd.clone(),
            usdc.clone(),
            factory.address.clone(),
            Vec::<(Address, i128)>::new(env),
        ),
    );
    let backstop = backstop::Client::new(env, &backstop);
    // No reserves are configured: this suite exercises backstop migration, not lending/oracle pricing.
    let pool = factory.deploy(
        admin,
        &String::from_str(env, name),
        &BytesN::from_array(env, &[0; 32]),
        &Address::generate(env),
        &1_000_000,
        &4,
        &0,
    );
    backstop.deposit(admin, &pool, &BACKSTOP_DEPOSIT);
    backstop.add_reward(&pool, &None);
    Stack {
        lp,
        backstop,
        factory,
        pool: pool::Client::new(env, &pool),
    }
}
