use super::*;

pub(super) fn scenario(version: i64) -> Start {
    Scenario::parse(include_str!(
        "../../../../examples/paper-learning-scenario.json"
    ))
    .unwrap()
    .start(
        "00000000-0000-4000-8000-000000000001".into(),
        version,
        Settings {
            quantity: 10,
            minimum_price: price("0"),
            maximum_price: price("100"),
            starting_balance: 100 * DOLLAR,
        },
        1795046400000,
    )
    .unwrap()
}

pub(super) fn saved(engine: &Engine) -> i64 {
    let text = include_str!("../../../../examples/learning.json");
    match engine
        .strategies(StrategyRequest::Save {
            json: text.into(),
            reviewed_json: text.into(),
        })
        .unwrap()
    {
        StrategyResponse::Saved { version } => version.id.parse().unwrap(),
        _ => panic!("save failed"),
    }
}
pub(super) struct Driver<'a> {
    pub(super) engine: &'a Engine,
    pub(super) context: InputContext,
    pub(super) ordinal: i64,
    pub(super) summary: RunSummary,
}
impl<'a> Driver<'a> {
    pub(super) fn start(engine: &'a Engine) -> Self {
        let reply = engine.paper_start(scenario(saved(engine))).unwrap();
        Self {
            engine,
            context: reply.context.unwrap(),
            ordinal: 0,
            summary: reply.summary,
        }
    }
    pub(super) fn input(&mut self, at: u64, payload: Payload) -> Input {
        let input = Input {
            context: self.context,
            ordinal: self.ordinal + 1,
            virtual_ms: at,
            payload,
        };
        let reply = self
            .engine
            .paper_input(input.clone())
            .unwrap_or_else(|error| {
                panic!(
                    "input {} at {} {:?}: {:?}",
                    input.ordinal, input.virtual_ms, input.payload, error
                )
            });
        self.ordinal += 1;
        self.context = reply.context.unwrap();
        self.summary = reply.summary;
        input
    }
    pub(super) fn source(&mut self, at: u64, event: src::SourceEvent) -> Input {
        self.input(
            at,
            Payload::Source(src::Envelope {
                clock: src::Clock {
                    at_ms: at,
                    receipt_ms: at,
                    received_through_ms: at,
                    completed_frontier_ms: self.summary.last_evaluated_through_ms.parse().unwrap(),
                },
                provider_identity: None,
                provider_time_utc: None,
                event,
            }),
        )
    }
    pub(super) fn update(&mut self, at: u64, initial: bool) {
        let mut updates = Vec::new();
        for (id, side, market, contract, value) in [
            (
                1,
                rules::Side::Yes,
                "KXBTC15M-PAPER-CURRENT",
                "paper-current",
                "35",
            ),
            (
                2,
                rules::Side::No,
                "KXBTC15M-PAPER-CURRENT",
                "paper-current",
                "70",
            ),
            (
                3,
                rules::Side::Yes,
                "KXBTC15M-PAPER-NEXT",
                "paper-next",
                if initial { "35" } else { "80" },
            ),
            (
                4,
                rules::Side::No,
                "KXBTC15M-PAPER-NEXT",
                "paper-next",
                if initial { "70" } else { "25" },
            ),
        ] {
            if !initial && id < 3 {
                continue;
            }
            updates.push(src::BindingUpdate {
                binding: rules::BindingDescriptor {
                    binding_id: id,
                    input_id: "selected_side_ask".into(),
                    scope: rules::BindingScope::Contract {
                        market_id: market.into(),
                        contract_id: contract.into(),
                        side,
                    },
                },
                unit: rules::RuleUnit::ContractCents,
                value: Some(if initial {
                    src::ValueUpdate::Initialize(price(value))
                } else {
                    src::ValueUpdate::Observation {
                        number: 1,
                        value: price(value),
                    }
                }),
                valid_until_ms: Some(1000000),
            });
        }
        self.source(
            at,
            src::SourceEvent::Update {
                stream: src::Stream::Book,
                token: self.context.book_token,
                updates,
                coverage: vec![],
            },
        );
    }
    /// Connects, verifies both markets, initializes their asks and enters NEXT at its exact
    /// opening (1000), latching the YES side.
    pub(super) fn enter(&mut self) {
        self.source(
            0,
            src::SourceEvent::Connection {
                provider: src::Provider::Kalshi,
                token: self.context.book_token,
                state: src::Connection::Open,
            },
        );
        for id in ["KXBTC15M-PAPER-CURRENT", "KXBTC15M-PAPER-NEXT"] {
            self.source(
                0,
                src::SourceEvent::Metadata {
                    token: self.context.metadata_token,
                    market_id: id.into(),
                    verified: true,
                },
            );
        }
        self.update(0, true);
        self.source(
            1000,
            src::SourceEvent::Boundary {
                market_id: "KXBTC15M-PAPER-NEXT".into(),
            },
        );
    }
    /// `enter`, then a qualifying opposite-side (NO 25) ask at 61000.
    pub(super) fn prepare(&mut self) {
        self.enter();
        self.update(61000, false);
    }
    /// A new observation of both next-market asks.
    #[allow(dead_code)] // Used by tests/paper.rs; lib.rs also includes this fixture.
    pub(super) fn observe(&mut self, at: u64, number: u64, yes: &str, no: &str) {
        self.send(at, yes, no, Some(number));
    }
    #[allow(dead_code)] // Used by tests/paper.rs; lib.rs also includes this fixture.
    fn send(&mut self, at: u64, yes: &str, no: &str, number: Option<u64>) {
        let updates = [(3, rules::Side::Yes, yes), (4, rules::Side::No, no)]
            .map(|(id, side, value)| src::BindingUpdate {
                binding: rules::BindingDescriptor {
                    binding_id: id,
                    input_id: "selected_side_ask".into(),
                    scope: rules::BindingScope::Contract {
                        market_id: "KXBTC15M-PAPER-NEXT".into(),
                        contract_id: "paper-next".into(),
                        side,
                    },
                },
                unit: rules::RuleUnit::ContractCents,
                value: Some(match number {
                    Some(number) => src::ValueUpdate::Observation {
                        number,
                        value: price(value),
                    },
                    None => src::ValueUpdate::Initialize(price(value)),
                }),
                valid_until_ms: Some(1000000),
            })
            .to_vec();
        self.source(
            at,
            src::SourceEvent::Update {
                stream: src::Stream::Book,
                token: self.context.book_token,
                updates,
                coverage: vec![],
            },
        );
    }
    /// Restates both next-market asks as initializations: a new value transaction that keeps
    /// each observation number, so it is not newer market data.
    #[allow(dead_code)] // Used by tests/paper.rs; lib.rs also includes this fixture.
    pub(super) fn reinitialize(&mut self, at: u64, yes: &str, no: &str) {
        self.send(at, yes, no, None);
    }
    /// A finite book bound to the latest observation of one next-market side.
    #[allow(dead_code)] // Used by tests/paper.rs; lib.rs also includes this fixture.
    pub(super) fn side_book(
        &self,
        side: rules::Side,
        ask: &str,
        number: u64,
        at: u64,
        quantity: u64,
    ) -> Book {
        Book {
            side,
            binding_id: if side == rules::Side::Yes { 3 } else { 4 },
            ask: price(ask),
            observation_number: number,
            accepted_ms: at,
            ..self.book(quantity)
        }
    }
    pub(super) fn book(&self, quantity: u64) -> Book {
        Book {
            source_market_id: "KXBTC15M-PAPER-NEXT".into(),
            source_contract_id: "paper-next".into(),
            side: rules::Side::No,
            binding_id: 4,
            source_generation: self.context.generation,
            value_transaction: self.context.last_value_transaction,
            observation_number: 1,
            ask: price("25"),
            available_millionths: quantity,
            accepted_ms: 61000,
            valid_until_ms: 1000000,
        }
    }
}
