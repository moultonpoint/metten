<h1>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/exports/svg/metten-a-lockup-reversed.svg">
  <img src="assets/brand/exports/svg/metten-a-lockup-primary.svg" alt="Metten: three offset rounded pieces with a cobalt center, beside the lowercase metten wordmark" width="256">
</picture>
</h1>

Metten is an open-source Mac app in development for running supported Kalshi strategies locally. It is designed around a shared engine that keeps strategy rules separate from purchasing, accounting, and recovery.

> [!NOTE]
> **Early development:** Metten currently offers a read-only Sources prototype, a non-trading test session with saved history, and local JSON strategy preview and immutable versions. A controlled paper learning example now connects saved rules to finite simulated purchases, exact fees, settlement and market history. It uses synthetic inputs and an accelerated clock. Live trading and a public release are not available.

[Controlled paper walkthrough](docs/strategies/paper.md) · [Write a strategy](docs/strategies/authoring.md) · [Agent authoring](docs/strategies/agent-authoring.md) · [Get started](#run-the-non-trading-foundation) · [Contributing](CONTRIBUTING.md) · [Product specification](METTEN_DESIGN_DRAFT_2026-11-12.md) · [Issues](https://github.com/moultonpoint/metten/issues)

## What we're building

The first version targets Bitcoin 15-minute markets, with independent paper runs and one live run. Strategies will use validated JSON rules, while the engine handles execution and safeguards. Paper is the default; real-money validation requires separate explicit authorization. Paper results do not prove live execution or future performance.

Metten is intended to keep credentials in local-only Mac Keychain and nonsecret structured data in SQLite, without a Metten account or telemetry. These are product requirements, not fully verified guarantees. See the [product specification](METTEN_DESIGN_DRAFT_2026-11-12.md) and [architecture decision](docs/adr/0001-rust-engine-and-tauri-boundaries.md) for details.

## Explore and contribute

Browse the [implementation plan](docs/implementation-plan.md) and [open issues](https://github.com/moultonpoint/metten/issues) to see what's planned and what is ready. The [project vocabulary](GLOSSARY.md) explains the terms used throughout the documentation.

See the [contribution guide](CONTRIBUTING.md) for setup, checks, and contribution boundaries. To report a vulnerability privately, use the [security policy](SECURITY.md).

## Repository map

- [`src/`](src/) — application interface.
- [`src-tauri/`](src-tauri/) — native Mac integration.
- [`crates/engine/`](crates/engine/) — shared engine and its tests.
- [`tests/`](tests/) — interface and integration checks.
- [`docs/`](docs/) — architecture decisions, implementation plan, and verification records.
- [`scripts/`](scripts/) — development and verification tools.
- [`assets/brand/`](assets/brand/README.md) — logo artwork and usage guidance.

## Run the non-trading foundation

Install Node.js 24 or later, the pinned Rust toolchain from `rust-toolchain.toml`, and Apple's Xcode Command Line Tools. Then:

```sh
npm ci
npm run tauri dev
```

Sources exposes named read-only connections for Kalshi, BRTI, Coinbase and Kraken. Ordinary setup uses production market data. To connect Kalshi, enter its API key ID and PEM private key in Sources and save them to protected, local-only Mac Keychain, then press Connect for the source you want. Saving a key or reopening the app does not connect automatically. BRTI shares the Kalshi credential; authentication alone does not prove BRTI access. Received prices do not establish verified freshness, complete history or strategy usability; inspect each source's status and the [source profiles and limits](docs/sources/README.md).

The Paper workspace runs the saved learning strategy with fixed settings, synthetic finite books and a 20× clock. It stays separate from Sources and makes no provider requests. Follow the [paper guide](docs/strategies/paper.md) for controls, fees, receipts and reopening.

The separate controlled session still uses a manual test clock: start, advance and pause it. Now and History display committed SQLite records; reopening retains history and starts stopped. This session is not paper trading and has no orders or financial accounting.

Build a local Mac application with `npm run tauri build -- --bundles app`; open `target/release/bundle/macos/Metten.app`. This local bundle is not a signed/notarized public release.

The default workspace is the operating system's application-data directory for `com.metten.foundation`. For isolated checks, launch the bundle's `Contents/MacOS/metten` executable with `METTEN_TEST_WORKSPACE` set to a fresh temporary directory. Never point checks at another application's workspace. No workspace path is accepted from the screen.

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run check
npm run build
```

See [the issue #2 verification record](docs/verification/issue-2.md) for exact local evidence and limitations.

## License

Licensed under the [MIT License](LICENSE).

Brand artwork and font provenance are documented in the [brand guide](assets/brand/README.md); those rights are not established by the software license.

## Implementation documentation and drift checks

The [EGM coverage map](docs/egm/README.md) links code-grounded component contracts through completed issue #4 and includes a repeatable drift-report request. These contracts cover the non-trading session, dummy credential integration and local strategy rules/version workflow; the product specification and accepted ADRs remain authoritative for intended behavior.

### Interface

- [Rust crates](crates/README.md) own controlled session state, rule validation/evaluation and immutable version recording; [native desktop integration](src-tauri/README.md) selects storage, validates requests and hosts the isolated credential proof; [the interface](src/README.md) displays native state and requests actions.
- [Tests](tests/README.md), [native verification tooling](scripts/README.md), [CI](.github/workflows/checks.yml), [assets](assets/README.md), and [documentation](docs/README.md) explain supporting boundaries and evidence.
- `package.json` defines `dev`, `check`, `build` and `tauri`. Development uses loopback port 5173 with `--strictPort`; checking performs TypeScript validation and JavaScript tests; building runs TypeScript and Vite. `tsconfig.json` checks `src` with strict ES2022/DOM settings and emits no TypeScript output.
- `Cargo.toml` groups `crates/engine` and `src-tauri` with resolver 2. `rust-toolchain.toml` pins Rust 1.100.0 with rustfmt/clippy. `Cargo.lock` and `package-lock.json` record resolved dependencies; use locked Cargo checks and `npm ci` for reproducible installation.

### Invariants

The screen does not own saved truth. Accepted requests are not confirmed writes; errors preserve last confirmed history. The current foundation has no trading operation. Ordinary source setup supports protected Kalshi credentials; isolated verification uses separate dummy credentials. The [issue #4 evidence](docs/verification/issue-4.md#current-evidence-index) records bounded local strategy checks. The controlled rule evaluator is separate from session Start/Pause; read-only source receipt does not enable paper/live execution or establish release readiness. The [issue #5 evidence](docs/verification/issue-5.md#current-evidence-index) records bounded Kalshi production receipt and controlled source checks; ordinary production Keychain/GUI operation and BRTI access remain unverified.

### Side Effects

Application execution can create a local workspace, SQLite history and lock files. Explicit Sources actions can store or change provider credentials in Mac Keychain and connect directly to named providers for read-only market data. The isolated native verifier can create and remove owned disposable Keychains and workspaces. Build/install/check commands can download dependencies and create generated files, binaries and test resources. See the owning component contract before running them. `.gitignore` excludes private planning/research, credentials, runtime databases, dependency directories and build/schema output; no such records belong in drift reports.

### Verification

Use the existing relevant checks above and in CONTRIBUTING.md for implementation changes. Documentation-only drift checks inspect source, inventory, references and links; they do not require launching the application or repeating unchanged packaged evidence. The [initial report](docs/egm/2026-11-13_DRIFT_REPORT.md) records this baseline's findings and limits.
