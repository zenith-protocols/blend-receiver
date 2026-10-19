pub mod emitter {
    soroban_sdk::contractimport!(file = "../wasm/emitter_v1.0.0.wasm");
}

pub mod receiver {
    soroban_sdk::contractimport!(file = "../wasm/blend_receiver.wasm");
}

pub mod backstop {
    soroban_sdk::contractimport!(file = "../wasm/backstop_v2.0.0.wasm");
}

#[allow(clippy::too_many_arguments)] // The official contract ABI determines argument counts.
pub mod pool_factory {
    soroban_sdk::contractimport!(file = "../wasm/pool_factory_v2.0.0.wasm");
}

#[allow(clippy::too_many_arguments)]
pub mod pool {
    soroban_sdk::contractimport!(file = "../wasm/pool_v2.0.0.wasm");
}

pub mod comet {
    soroban_sdk::contractimport!(file = "../wasm/comet_v1.1.0.wasm");
    pub const LEGACY_WASM: &[u8] = include_bytes!("../../wasm/comet_v1.0.0.wasm");
}

pub mod test_fixture;
