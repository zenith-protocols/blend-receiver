# WASM dependencies

These are unmodified official release artifacts from Blend Capital and Comet
DEX, distributed under the GNU Affero General Public License v3.0. The
upstream license is included in [LICENSE](LICENSE). The receiver source has
its own [MIT license](../LICENSE).

[sources.json](sources.json) records each dependency's original download URL,
SHA256 and size. The corresponding source, build configuration and release
are available at these upstream tags:

| Artifact | Corresponding source |
| --- | --- |
| `emitter_v1.0.0.wasm` | [Blend emitter v1.0.0](https://github.com/blend-capital/blend-contracts/tree/v1.0.0_emitter_v1.0.0.wasm) |
| `backstop_v2.0.0.wasm` | [Blend backstop v2.0.0](https://github.com/blend-capital/blend-contracts-v2/tree/v2.0.0_backstop_cli22.0.1) |
| `pool_v2.0.0.wasm` | [Blend pool v2.0.0](https://github.com/blend-capital/blend-contracts-v2/tree/v2.0.0_pool_cli22.0.1) |
| `pool_factory_v2.0.0.wasm` | [Blend pool factory v2.0.0](https://github.com/blend-capital/blend-contracts-v2/tree/v2.0.0_pool-factory_cli22.0.1) |
| `comet_v1.0.0.wasm` | [Comet v1.0.0](https://github.com/CometDEX/comet-contracts-v1/tree/v1.0.0_contracts_v1.0.0.wasm) |
| `comet_v1.1.0.wasm` | [Comet v1.1.0](https://github.com/CometDEX/comet-contracts-v1/tree/v1.1.0_contracts_pkg1.0.0_cli25.1.0) |

`blend_receiver.wasm` is generated from this repository by `make build` or
`make test` and is ignored by Git.
