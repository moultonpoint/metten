use metten_engine::sources::*;
use metten_engine::strategy::*;
use serde_json::json;

fn rules() -> ValidatedStrategy {
    ValidatedStrategy::parse(&json!({"format":2,"name":"Controlled source asks","inputs":[{"id":"ask","source":"kalshi","instrument":"current_contract","field":"buyable_ask","unit":"contract_cents","side":"selected_side"}],"checkpoints":[{"id":"equal","when":{"type":"compare","left":{"reference":"ask"},"comparison":"equal_to","right":{"value":"40","unit":"contract_cents"}},"remember_side":{"type":"fixed","side":"yes"}}],"purchase_options":[{"id":"buy","side":{"type":"fixed","side":"yes"},"when":{"type":"compare","left":{"reference":"ask"},"comparison":"less_than","right":{"value":"40","unit":"contract_cents"}}}]}).to_string()).unwrap()
}
fn market(id: &str, open: i64) -> MarketDescriptor {
    MarketDescriptor {
        series_id: "btc15m".into(),
        market_id: id.into(),
        contract_id: format!("contract_{id}"),
        open_ms: open,
        close_ms: open + 900000,
    }
}
fn binding(id: u32, m: &str, side: Side) -> BindingDescriptor {
    BindingDescriptor {
        binding_id: id,
        input_id: "ask".into(),
        scope: BindingScope::Contract {
            market_id: m.into(),
            contract_id: format!("contract_{m}"),
            side,
        },
    }
}
fn controlled() -> Capabilities {
    Capabilities {
        current: None,
        observations: None,
        history: None,
    }
}
fn clock(at: u64) -> Clock {
    Clock {
        at_ms: at,
        receipt_ms: at,
        received_through_ms: at,
        completed_frontier_ms: 0,
    }
}
fn envelope(at: u64, event: SourceEvent) -> Envelope {
    Envelope {
        clock: clock(at),
        provider_identity: None,
        provider_time_utc: None,
        event,
    }
}
fn connect(s: &mut SourceState, at: u64) {
    s.apply(envelope(
        at,
        SourceEvent::Connection {
            token: s.token(Stream::Book),
            provider: Provider::Kalshi,
            state: Connection::Open,
        },
    ))
    .unwrap();
}
fn state(two: bool) -> SourceState {
    let mut markets = vec![market("m", 1)];
    let mut bindings = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
    if two {
        markets.push(market("n", 900001));
        bindings.extend([binding(3, "n", Side::Yes), binding(4, "n", Side::No)]);
    }
    let caps: Vec<_> = bindings
        .iter()
        .map(|b| (b.binding_id, controlled()))
        .collect();
    let mut s = SourceState::new(&rules(), &markets, &bindings, &caps).unwrap();
    connect(&mut s, 0);
    s
}
fn update(b: BindingDescriptor, v: &str, number: Option<u64>, until: u64) -> BindingUpdate {
    BindingUpdate {
        binding: b,
        unit: RuleUnit::ContractCents,
        value: Some(match number {
            None => ValueUpdate::Initialize(Decimal::parse(v).unwrap()),
            Some(number) => ValueUpdate::Observation {
                number,
                value: Decimal::parse(v).unwrap(),
            },
        }),
        valid_until_ms: Some(until),
    }
}
fn values(s: &mut SourceState, at: u64, updates: Vec<BindingUpdate>) -> SourceEffect {
    s.apply(envelope(
        at,
        SourceEvent::Update {
            stream: Stream::Book,
            token: s.token(Stream::Book),
            updates,
            coverage: vec![],
        },
    ))
    .unwrap()
}
fn initialize(s: &mut SourceState, two: bool, at: u64) -> SourceEffect {
    let mut updates = vec![
        update(binding(1, "m", Side::Yes), "30", None, 2000000),
        update(binding(2, "m", Side::No), "60", None, 2000000),
    ];
    if two {
        updates.extend([
            update(binding(3, "n", Side::Yes), "31", None, 2000000),
            update(binding(4, "n", Side::No), "61", None, 2000000),
        ]);
    }
    values(s, at, updates)
}
fn verify(s: &mut SourceState, id: &str, at: u64) -> SourceEffect {
    s.apply(envelope(
        at,
        SourceEvent::Metadata {
            token: s.token(Stream::Metadata),
            market_id: id.into(),
            verified: true,
        },
    ))
    .unwrap()
}
fn finish(e: &mut Evaluator, effect: SourceEffect) -> Decision {
    finish_with_objective(e, effect, None)
}
fn objective() -> ObjectiveView {
    let range = PriceRange {
        minimum: Decimal::parse("1").unwrap(),
        maximum: Decimal::parse("99").unwrap(),
    };
    ObjectiveView {
        market_id: "m".into(),
        requested_quantity: 1,
        confirmed_quantity: 0,
        locked_side: None,
        attempt_state: AttemptState::None,
        safeguards: Safeguards {
            allowed: true,
            yes: range.clone(),
            no: range,
        },
    }
}
fn finish_with_objective(
    e: &mut Evaluator,
    effect: SourceEffect,
    objective: Option<ObjectiveView>,
) -> Decision {
    e.begin(effect.batch).unwrap();
    loop {
        match e.process_slice(SliceControl::Continue, objective.clone()) {
            SliceResult::Pending { .. } => (),
            SliceResult::Complete(d) => return d,
            other => panic!("{other:?}"),
        }
    }
}
fn evaluator() -> Evaluator {
    Evaluator::new(
        rules(),
        RunContext {
            strategy_version: "v1".into(),
            run_id: "controlled".into(),
            go_time: "2026-11-15T00:00:00Z".into(),
            series_id: "btc15m".into(),
            markets: vec![market("m", 1)],
            bindings: vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)],
        },
    )
    .unwrap()
}

#[test]
fn acknowledgment_without_data_and_unsupported_profiles_never_grant_usability() {
    let mut s = SourceState::new(
        &rules(),
        &[market("m", 1)],
        &[binding(1, "m", Side::Yes), binding(2, "m", Side::No)],
        &[],
    )
    .unwrap();
    connect(&mut s, 0);
    s.apply(envelope(
        0,
        SourceEvent::Subscription {
            stream: Stream::Book,
            token: s.token(Stream::Book),
            state: Subscription::Acknowledged,
        },
    ))
    .unwrap();
    assert!(
        s.snapshot(0)
            .unwrap()
            .bindings
            .iter()
            .all(|b| !b.initialized && !b.usable)
    );
    let effect = initialize(&mut s, false, 0);
    assert!(
        effect.batch.transactions[0]
            .changes
            .iter()
            .all(|c| matches!(c, EvidenceChange::Resynchronize { .. }))
    );
    assert!(
        s.snapshot(0)
            .unwrap()
            .bindings
            .iter()
            .all(|b| b.value.is_some()
                && !b.usable
                && b.capabilities.history == Some(Unavailable::History))
    );
    verify(&mut s, "m", 0);
    assert_ne!(
        s.snapshot(0).unwrap().markets[0].preparation,
        Preparation::Ready
    );
}
#[test]
fn shared_subscription_loss_revokes_current_and_successor_repeatedly() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "m", 0);
    verify(&mut s, "n", 0);
    assert!(
        s.snapshot(0)
            .unwrap()
            .markets
            .iter()
            .all(|m| m.preparation == Preparation::Ready)
    );
    let old = s.token(Stream::Book);
    let before = s.stamp(&[1, 3]).unwrap();
    let loss = s
        .invalidate(
            Scope::Stream(Stream::Book),
            Unavailable::Gap,
            1,
            Some(GapInterval {
                start_ms: 0,
                end_ms: 1,
            }),
        )
        .unwrap();
    assert_eq!(loss.batch.transactions.len(), 2);
    assert!(!s.is_current(&before));
    assert!(s.snapshot(1).unwrap().bindings.iter().all(|b| !b.usable));
    assert_eq!(
        s.apply(envelope(
            1,
            SourceEvent::Update {
                stream: Stream::Book,
                token: old,
                updates: vec![update(binding(3, "n", Side::Yes), "31", None, 2000000)],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "obsolete_generation"
    );
    values(
        &mut s,
        1,
        vec![
            update(binding(1, "m", Side::Yes), "30", None, 2000000),
            update(binding(2, "m", Side::No), "60", None, 2000000),
        ],
    );
    let view = s.snapshot(1).unwrap();
    assert!(view.bindings[..2].iter().all(|b| b.usable));
    assert!(view.bindings[2..].iter().all(|b| !b.usable));
    values(
        &mut s,
        1,
        vec![
            update(binding(3, "n", Side::Yes), "31", None, 2000000),
            update(binding(4, "n", Side::No), "61", None, 2000000),
        ],
    );
    assert!(
        s.snapshot(1)
            .unwrap()
            .markets
            .iter()
            .all(|m| m.preparation == Preparation::Ready)
    );
    s.invalidate(Scope::Stream(Stream::Book), Unavailable::Gap, 2, None)
        .unwrap();
    assert!(
        s.snapshot(2)
            .unwrap()
            .markets
            .iter()
            .all(|m| m.preparation != Preparation::Ready)
    );
}
#[test]
fn initialization_replay_renewal_and_equal_price_observations_have_distinct_meaning() {
    let mut s = state(false);
    let mut e = evaluator();
    finish(&mut e, initialize(&mut s, false, 0));
    let same = values(
        &mut s,
        0,
        vec![update(binding(1, "m", Side::Yes), "30", Some(1), 100)],
    );
    assert!(matches!(
        same.batch.transactions[0].changes[0],
        EvidenceChange::Observation {
            observation_number: 1,
            ..
        }
    ));
    finish(&mut e, same);
    let second = values(
        &mut s,
        0,
        vec![update(binding(1, "m", Side::Yes), "30", Some(2), 100)],
    );
    assert!(matches!(
        second.batch.transactions[0].changes[0],
        EvidenceChange::Observation {
            observation_number: 2,
            ..
        }
    ));
    finish(&mut e, second);
    let replay = values(
        &mut s,
        0,
        vec![update(binding(1, "m", Side::Yes), "30", Some(2), 100)],
    );
    assert!(
        replay.batch.transactions[0]
            .changes
            .iter()
            .all(|c| matches!(c, EvidenceChange::Validity { .. }))
    );
    let lease = BindingUpdate {
        binding: binding(1, "m", Side::Yes),
        unit: RuleUnit::ContractCents,
        value: None,
        valid_until_ms: Some(200),
    };
    let renewed = values(&mut s, 1, vec![lease.clone()]);
    assert!(matches!(
        renewed.batch.transactions[0].changes[0],
        EvidenceChange::Validity {
            valid_until_ms: 200,
            ..
        }
    ));
    assert!(s.snapshot(1).unwrap().bindings[0].usable);
    s.invalidate(
        Scope::Stream(Stream::Book),
        Unavailable::Disconnected,
        2,
        None,
    )
    .unwrap();
    assert_eq!(
        s.apply(envelope(
            2,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![lease],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "invalid_request"
    );
    assert!(!s.snapshot(2).unwrap().bindings[0].usable);
    values(
        &mut s,
        2,
        vec![update(binding(1, "m", Side::Yes), "31", None, 200)],
    );
    assert!(s.snapshot(2).unwrap().bindings[0].usable);
    assert!(!s.snapshot(200).unwrap().bindings[0].usable);
}
#[test]
fn wrong_identity_invalid_values_and_conflicts_cannot_contaminate_bindings() {
    let mut s = state(false);
    initialize(&mut s, false, 0);
    let mut wrong = update(binding(1, "m", Side::No), "30", Some(1), 100);
    assert_eq!(
        s.apply(envelope(
            1,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![wrong.clone()],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "identity_mismatch"
    );
    assert!(s.snapshot(1).unwrap().bindings[0].usable);
    wrong.binding = binding(1, "m", Side::Yes);
    wrong.unit = RuleUnit::UsdPerBtc;
    assert_eq!(
        s.apply(envelope(
            1,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![wrong],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "identity_mismatch"
    );
    assert!(Decimal::parse("68000.1234567").is_err());
    let invalid = update(binding(1, "m", Side::Yes), "100.000001", Some(1), 100);
    assert_eq!(
        s.apply(envelope(
            1,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![invalid],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "invalid_value"
    );
    assert!(s.snapshot(1).unwrap().bindings.iter().all(|b| !b.usable));
    s.invalidate(
        Scope::Stream(Stream::Book),
        Unavailable::InvalidValue,
        1,
        None,
    )
    .unwrap();
    initialize(&mut s, false, 1);
    values(
        &mut s,
        1,
        vec![update(binding(1, "m", Side::Yes), "30", Some(1), 100)],
    );
    assert_eq!(
        s.apply(envelope(
            2,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![update(binding(1, "m", Side::Yes), "31", Some(1), 100)],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "inconsistent_evidence"
    );
    assert!(s.snapshot(2).unwrap().bindings.iter().all(|b| !b.usable));
}
#[test]
fn market_handoff_is_once_only_with_retirement_and_late_display_is_not_entry() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "m", 0);
    verify(&mut s, "n", 0);
    let first = s
        .apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap();
    assert_eq!(first.batch.transactions.len(), 1);
    let old = s.stamp(&[1]).unwrap();
    assert!(
        s.apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into()
            }
        ))
        .unwrap()
        .batch
        .transactions
        .is_empty()
    );
    let next = s
        .apply(envelope(
            900001,
            SourceEvent::Boundary {
                market_id: "n".into(),
            },
        ))
        .unwrap();
    assert_eq!(next.batch.transactions.len(), 1);
    assert!(!s.is_current(&old));
    let view = s.snapshot(900001).unwrap();
    assert_eq!(view.current_market.as_deref(), Some("n"));
    assert_eq!(view.markets.len(), 1);
    assert_eq!(view.bindings.len(), 2);
    assert_eq!(
        s.apply(envelope(
            900001,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![update(binding(1, "m", Side::Yes), "1", Some(99), 2000000)],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "identity_mismatch"
    );
    let mut late = state(false);
    initialize(&mut late, false, 0);
    verify(&mut late, "m", 0);
    late.apply(envelope(2, SourceEvent::Advance)).unwrap();
    assert!(
        late.apply(envelope(
            2,
            SourceEvent::Boundary {
                market_id: "m".into()
            }
        ))
        .unwrap()
        .batch
        .transactions
        .is_empty()
    );
    assert_eq!(
        late.snapshot(2).unwrap().current_market.as_deref(),
        Some("m")
    );
    let mut missing = state(false);
    initialize(&mut missing, false, 0);
    assert!(
        missing
            .apply(envelope(
                1,
                SourceEvent::Boundary {
                    market_id: "m".into()
                }
            ))
            .unwrap()
            .batch
            .transactions
            .is_empty()
    );
    assert_eq!(missing.snapshot(1).unwrap().current_market, None);
}
#[test]
fn dependency_stamps_stop_owner_identity_and_clock_fail_closed() {
    let mut s = state(false);
    initialize(&mut s, false, 0);
    let stamp = s.stamp(&[1, 2]).unwrap();
    s.invalidate(
        Scope::Stream(Stream::Coinbase),
        Unavailable::Overflow,
        1,
        None,
    )
    .unwrap();
    assert!(s.is_current(&stamp));
    s.invalidate(
        Scope::Provider(Provider::Kalshi),
        Unavailable::OverAge,
        2,
        None,
    )
    .unwrap();
    assert!(!s.is_current(&stamp));
    initialize(&mut s, false, 2);
    let stamp = s.stamp(&[1]).unwrap();
    assert!(!state(false).is_current(&stamp));
    assert_eq!(s.snapshot(1).unwrap_err().code, "invalid_request");
    let mut invalid = envelope(3, SourceEvent::Advance);
    invalid.clock.completed_frontier_ms = 4;
    assert_eq!(s.apply(invalid).unwrap_err().code, "invalid_order");
    assert!(s.is_current(&stamp));
    s.stop(3).unwrap();
    assert!(!s.is_current(&stamp));
    assert!(s.snapshot(3).unwrap().bindings.iter().all(|b| !b.usable));
    assert_eq!(
        s.apply(envelope(4, SourceEvent::Advance)).unwrap_err().code,
        "stopped"
    );
    assert!(s.stop(4).unwrap().batch.transactions.is_empty());
}
#[test]
fn one_coherent_update_stays_atomic_through_the_public_source_and_evaluator() {
    let mut s = state(false);
    let mut e = evaluator();
    finish(&mut e, initialize(&mut s, false, 0));
    verify(&mut s, "m", 0);
    finish(
        &mut e,
        s.apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap(),
    );
    let effect = values(
        &mut s,
        2,
        vec![
            update(binding(1, "m", Side::Yes), "30", Some(1), 100),
            update(binding(2, "m", Side::No), "30", Some(1), 100),
        ],
    );
    assert_eq!(effect.batch.transactions.len(), 1);
    assert_eq!(effect.batch.transactions[0].changes.len(), 4);
    assert!(!finish(&mut e, effect).checkpoints.contains_key("equal"));
}
#[test]
fn shared_connection_denial_and_disconnection_are_not_absent_data() {
    let mut s = state(false);
    initialize(&mut s, false, 0);
    s.apply(envelope(
        1,
        SourceEvent::Connection {
            token: s.token(Stream::Book),
            provider: Provider::Kalshi,
            state: Connection::AccessRefused,
        },
    ))
    .unwrap();
    let view = s.snapshot(1).unwrap();
    assert_eq!(view.sources[0], (Source::Kalshi, Connection::AccessRefused));
    assert_eq!(view.sources[1], (Source::Brti, Connection::AccessRefused));
    assert_eq!(view.bindings[0].reason, Some(Unavailable::Access));
    assert!(!view.bindings[0].usable);
}

#[test]
fn historical_coverage_is_explicit_and_recovery_does_not_fill_a_hole() {
    let mut r = serde_json::from_str::<serde_json::Value>(rules().json()).unwrap();
    r["measurements"] = json!([{"id":"average","operation":"rolling_average","of":{"reference":"ask"},"lookback_seconds":1}]);
    let strategy = ValidatedStrategy::parse(&r.to_string()).unwrap();
    let bs = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
    let caps = vec![(1, controlled()), (2, controlled())];
    let mut s = SourceState::new(&strategy, &[market("m", 2001)], &bs, &caps).unwrap();
    connect(&mut s, 0);
    initialize(&mut s, false, 0);
    assert!(!s.snapshot(1000).unwrap().bindings[0].history_ready);
    let cover = |s: &mut SourceState, start, end| {
        s.apply(envelope(
            end,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![],
                coverage: vec![CoverageFact {
                    binding_id: 1,
                    start_ms: start,
                    end_ms: end,
                }],
            },
        ))
    };
    let effect = cover(&mut s, 0, 1000).unwrap();
    assert_eq!(effect.batch.coverage.len(), 1);
    assert!(s.snapshot(1000).unwrap().bindings[0].history_ready);
    let loss = s
        .invalidate(
            Scope::Stream(Stream::Book),
            Unavailable::Gap,
            2000,
            Some(GapInterval {
                start_ms: 1000,
                end_ms: 2000,
            }),
        )
        .unwrap();
    assert!(matches!(
        loss.batch.transactions[1].changes[0],
        EvidenceChange::Gap {
            start_ms: 1000,
            end_ms: 2000,
            ..
        }
    ));
    initialize(&mut s, false, 2000);
    assert!(!s.snapshot(2000).unwrap().bindings[0].history_ready);
    assert_eq!(cover(&mut s, 0, 2000).unwrap_err().code, "invalid_request");
    cover(&mut s, 2000, 3000).unwrap();
    assert!(s.snapshot(3000).unwrap().bindings[0].history_ready);
}

#[test]
fn multi_source_stamps_include_all_dependencies_and_shared_kalshi_delivery() {
    let mut r = serde_json::from_str::<serde_json::Value>(rules().json()).unwrap();
    r["inputs"].as_array_mut().unwrap().extend([
        json!({"id":"index","source":"brti","instrument":"btc_usd","field":"index_value","unit":"usd_per_btc"}),
        json!({"id":"spot","source":"coinbase","instrument":"btc_usd","field":"last_trade","unit":"usd_per_btc"})]);
    let strategy = ValidatedStrategy::parse(&r.to_string()).unwrap();
    let mut bs = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
    bs.extend([
        BindingDescriptor {
            binding_id: 3,
            input_id: "index".into(),
            scope: BindingScope::Underlying,
        },
        BindingDescriptor {
            binding_id: 4,
            input_id: "spot".into(),
            scope: BindingScope::Underlying,
        },
    ]);
    let mut s = SourceState::new(
        &strategy,
        &[market("m", 1)],
        &bs,
        &[
            (1, controlled()),
            (2, controlled()),
            (3, controlled()),
            (4, controlled()),
        ],
    )
    .unwrap();
    connect(&mut s, 0);
    let mixed = s.stamp(&[1, 3]).unwrap();
    let independent = s.stamp(&[4]).unwrap();
    s.invalidate(Scope::Source(Source::Kalshi), Unavailable::Gap, 0, None)
        .unwrap();
    assert!(!s.is_current(&mixed));
    assert!(s.is_current(&independent));
    let brti = s.stamp(&[3]).unwrap();
    s.invalidate(
        Scope::Provider(Provider::Kalshi),
        Unavailable::Disconnected,
        1,
        None,
    )
    .unwrap();
    assert!(!s.is_current(&brti));
    assert!(s.is_current(&independent));
}

#[test]
fn repeated_source_handoffs_reuse_bounded_slots_without_revalidating_old_stamps() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "m", 0);
    verify(&mut s, "n", 0);
    s.apply(envelope(
        1,
        SourceEvent::Boundary {
            market_id: "m".into(),
        },
    ))
    .unwrap();
    let old = s.stamp(&[1]).unwrap();
    s.apply(envelope(
        900001,
        SourceEvent::Boundary {
            market_id: "n".into(),
        },
    ))
    .unwrap();
    for n in 2..22 {
        let id = format!("m{n}");
        let at = (n - 1) * 900000 + 1;
        let open = n * 900000 + 1;
        let base = if n % 2 == 0 { 1 } else { 3 };
        let bs = vec![
            binding(base, &id, Side::Yes),
            binding(base + 1, &id, Side::No),
        ];
        s.apply(envelope(
            at,
            SourceEvent::RegisterMarket {
                token: s.token(Stream::Metadata),
                descriptor: market(&id, open as i64),
                bindings: bs.clone(),
                capabilities: vec![(base, controlled()), (base + 1, controlled())],
            },
        ))
        .unwrap();
        values(
            &mut s,
            at,
            vec![
                update(bs[0].clone(), "30", None, open + 900000),
                update(bs[1].clone(), "60", None, open + 900000),
            ],
        );
        verify(&mut s, &id, at);
        s.apply(envelope(
            open,
            SourceEvent::Boundary {
                market_id: id.clone(),
            },
        ))
        .unwrap();
        let view = s.snapshot(open).unwrap();
        assert_eq!(view.markets.len(), 1);
        assert_eq!(view.bindings.len(), 2);
        assert_eq!(view.current_market.as_deref(), Some(id.as_str()));
        assert!(!s.is_current(&old));
    }
}

#[test]
fn source_accepted_evidence_is_revoked_when_evaluator_admission_refuses_it() {
    let mut s = state(false);
    let mut e = evaluator();
    finish(&mut e, initialize(&mut s, false, 0));
    let mut effect = values(
        &mut s,
        0,
        vec![update(binding(1, "m", Side::Yes), "30", Some(1), 100)],
    );
    let stamp = s.stamp(&[1, 2]).unwrap();
    // The consumer's spare ingress allocation fits alone, not with evaluator metadata.
    let mut transactions =
        Vec::with_capacity((8 * 1024 * 1024 - 2048) / std::mem::size_of::<NormalizedTransaction>());
    transactions.append(&mut effect.batch.transactions);
    effect.batch.transactions = transactions;
    assert!(effect.batch.ingress_bytes().unwrap() < 8 * 1024 * 1024);
    assert_eq!(e.begin(effect.batch).unwrap_err().code, "resource_limit");
    let revoked = s
        .invalidate(Scope::All, Unavailable::Overflow, 0, None)
        .unwrap();
    assert!(!s.is_current(&stamp));
    finish(&mut e, revoked);
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| !b.usable));
    let recovered = initialize(&mut s, false, 0);
    finish(&mut e, recovered);
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| b.usable));
}

#[test]
fn invalidation_while_pending_and_busy_protocol_stop_never_publish_old_results() {
    let mut s = state(false);
    let mut e = evaluator();
    let effect = initialize(&mut s, false, 0);
    let stamp = s.stamp(&[1, 2]).unwrap();
    e.begin(effect.batch).unwrap();
    let revoked = s
        .invalidate(Scope::Stream(Stream::Book), Unavailable::OverAge, 1, None)
        .unwrap();
    assert!(!s.is_current(&stamp));
    loop {
        match e.process_slice(SliceControl::Continue, None) {
            SliceResult::Pending { .. } => (),
            SliceResult::Complete(_) => break,
            other => panic!("{other:?}"),
        }
    }
    assert!(!s.is_current(&stamp));
    finish(&mut e, revoked);
    let effect = initialize(&mut s, false, 1);
    e.begin(effect.batch).unwrap();
    let extra = s.apply(envelope(1, SourceEvent::Advance)).unwrap();
    assert_eq!(e.begin(extra.batch).unwrap_err().code, "busy");
    s.stop(1).unwrap();
    assert!(matches!(
        e.process_slice(SliceControl::Stop, None),
        SliceResult::Stopped
    ));
    assert!(s.snapshot(1).unwrap().bindings.iter().all(|b| !b.usable));
    assert!(!s.is_current(&stamp));
}

#[test]
fn public_registration_checks_match_engine_side_and_market_rules() {
    let strategy = rules();
    let markets = vec![market("m", 1)];
    let admitted = |strategy: &ValidatedStrategy, bindings: Vec<BindingDescriptor>| {
        (
            SourceState::new(strategy, &markets, &bindings, &[]).is_ok(),
            Evaluator::new(
                strategy.clone(),
                RunContext {
                    strategy_version: "v".into(),
                    run_id: "r".into(),
                    go_time: "controlled".into(),
                    series_id: "btc15m".into(),
                    markets: markets.clone(),
                    bindings,
                },
            )
            .is_ok(),
        )
    };
    for scope in [
        BindingScope::Underlying,
        BindingScope::Market {
            market_id: "m".into(),
        },
        BindingScope::Contract {
            market_id: "m".into(),
            contract_id: "wrong".into(),
            side: Side::Yes,
        },
    ] {
        assert_eq!(
            admitted(
                &strategy,
                vec![BindingDescriptor {
                    binding_id: 1,
                    input_id: "ask".into(),
                    scope
                }]
            ),
            (false, false)
        );
    }
    for (side, opposite, implicit) in [
        (Side::Yes, Side::No, "__buyable_ask_no"),
        (Side::No, Side::Yes, "__buyable_ask_yes"),
    ] {
        let mut r = serde_json::from_str::<serde_json::Value>(strategy.json()).unwrap();
        r["inputs"][0]["side"] = json!(if side == Side::Yes { "yes" } else { "no" });
        let strategy = ValidatedStrategy::parse(&r.to_string()).unwrap();
        let mut generated = binding(2, "m", opposite);
        generated.input_id = implicit.into();
        assert_eq!(
            admitted(&strategy, vec![binding(1, "m", side), generated]),
            (true, true)
        );
        let mut wrong = binding(9, "m", side);
        wrong.input_id = implicit.into();
        assert_eq!(admitted(&strategy, vec![wrong]), (false, false));
    }
}

#[test]
fn limits_bound_retained_records_views_and_owned_input_capacity() {
    let mut r = serde_json::from_str::<serde_json::Value>(rules().json()).unwrap();
    r["inputs"]=json!((0..32).map(|n|json!({"id":format!("i{n:063}"),"source":"kalshi","instrument":"current_contract","field":"buyable_ask","unit":"contract_cents","side":"selected_side"})).collect::<Vec<_>>());
    r["checkpoints"] = json!([]);
    r["purchase_options"][0]["when"]["left"]["reference"] = json!(format!("i{:063}", 0));
    let strategy = ValidatedStrategy::parse(&r.to_string()).unwrap();
    let markets: Vec<_> = (0..2)
        .map(|n| MarketDescriptor {
            series_id: "btc15m".into(),
            market_id: format!("{n}{}", "\"".repeat(127)),
            contract_id: format!("{n}{}", "\\".repeat(127)),
            open_ms: n * 900000 + 1,
            close_ms: (n + 1) * 900000 + 1,
        })
        .collect();
    let mut bs = Vec::new();
    for (m, market) in markets.iter().enumerate() {
        for input in 0..32 {
            for (side_index, side) in [Side::Yes, Side::No].into_iter().enumerate() {
                bs.push(BindingDescriptor {
                    binding_id: (m * 64 + input * 2 + side_index) as u32,
                    input_id: format!("i{input:063}"),
                    scope: BindingScope::Contract {
                        market_id: market.market_id.clone(),
                        contract_id: market.contract_id.clone(),
                        side,
                    },
                });
            }
        }
    }
    let caps: Vec<_> = bs.iter().map(|b| (b.binding_id, controlled())).collect();
    let mut s = SourceState::new(&strategy, &markets, &bs, &caps).unwrap();
    connect(&mut s, 0);
    assert_eq!(s.snapshot(0).unwrap().bindings.len(), 128);
    let effect = s
        .apply(envelope(
            0,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: bs
                    .iter()
                    .map(|b| update(b.clone(), "30", None, 2000000))
                    .collect(),
                coverage: vec![],
            },
        ))
        .unwrap();
    assert_eq!(effect.batch.transactions[0].changes.len(), 256);
    assert!(s.snapshot(0).is_ok());
    let oversized = envelope(
        0,
        SourceEvent::Update {
            stream: Stream::Book,
            token: s.token(Stream::Book),
            updates: Vec::with_capacity(129),
            coverage: vec![],
        },
    );
    assert_eq!(s.apply(oversized).unwrap_err().code, "resource_limit");
    let mut oversized = envelope(
        0,
        SourceEvent::Update {
            stream: Stream::Book,
            token: s.token(Stream::Book),
            updates: bs
                .iter()
                .map(|b| update(b.clone(), "30", None, 2000000))
                .collect(),
            coverage: vec![],
        },
    );
    oversized.provider_identity = Some("\\".repeat(128));
    oversized.provider_time_utc = Some("2026-11-15T23:59:59Z".into());
    s.apply(oversized).unwrap();
    assert_eq!(s.snapshot(0).unwrap_err().code, "resource_limit");
    let mut more = bs.clone();
    more.push(BindingDescriptor {
        binding_id: 200,
        input_id: bs[0].input_id.clone(),
        scope: bs[0].scope.clone(),
    });
    assert!(matches!(
        SourceState::new(&strategy, &markets, &more, &caps),
        Err(SourceFailure {
            code: "resource_limit",
            ..
        })
    ));
}

#[test]
fn obsolete_connection_metadata_and_discovery_results_cannot_restore_state() {
    let mut s = state(false);
    initialize(&mut s, false, 0);
    verify(&mut s, "m", 0);
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::Metadata {
                token: s.token(Stream::Book),
                market_id: "m".into(),
                verified: true
            }
        ))
        .unwrap_err()
        .code,
        "obsolete_generation"
    );
    let book = s.token(Stream::Book);
    let metadata = s.token(Stream::Metadata);
    s.invalidate(
        Scope::Provider(Provider::Kalshi),
        Unavailable::Disconnected,
        1,
        None,
    )
    .unwrap();
    let events = [
        SourceEvent::Connection {
            provider: Provider::Kalshi,
            token: book,
            state: Connection::Open,
        },
        SourceEvent::Metadata {
            token: metadata,
            market_id: "m".into(),
            verified: true,
        },
        SourceEvent::RegisterMarket {
            token: metadata,
            descriptor: market("n", 900001),
            bindings: vec![binding(3, "n", Side::Yes), binding(4, "n", Side::No)],
            capabilities: vec![(3, controlled()), (4, controlled())],
        },
    ];
    for event in events {
        assert_eq!(
            s.apply(envelope(1, event)).unwrap_err().code,
            "obsolete_generation"
        );
    }
    assert!(s.snapshot(1).unwrap().bindings.iter().all(|b| !b.usable));
    assert_eq!(s.snapshot(1).unwrap().markets.len(), 1);
}

#[test]
fn later_evaluator_registration_rejection_revokes_source_claims() {
    let mut s = state(false);
    let strategy = rules();
    let mut conflict = market("n", 900001);
    conflict.contract_id = "already_registered_contract".into();
    let mut e = Evaluator::new(
        strategy,
        RunContext {
            strategy_version: "v".into(),
            run_id: "r".into(),
            go_time: "controlled".into(),
            series_id: "btc15m".into(),
            markets: vec![market("m", 1), conflict],
            bindings: vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)],
        },
    )
    .unwrap();
    let effect = s
        .apply(envelope(
            0,
            SourceEvent::RegisterMarket {
                token: s.token(Stream::Metadata),
                descriptor: market("n", 900001),
                bindings: vec![binding(3, "n", Side::Yes), binding(4, "n", Side::No)],
                capabilities: vec![(3, controlled()), (4, controlled())],
            },
        ))
        .unwrap();
    let stamp = s.stamp(&[3, 4]).unwrap();
    e.begin(effect.batch).unwrap();
    loop {
        match e.process_slice(SliceControl::Continue, None) {
            SliceResult::Pending { .. } => (),
            SliceResult::Rejected(err) => {
                assert_eq!(err.code, "incompatible_market");
                break;
            }
            other => panic!("{other:?}"),
        }
    }
    let mut revoked = s
        .invalidate(Scope::Market, Unavailable::Inconsistent, 0, None)
        .unwrap();
    assert!(!s.is_current(&stamp));
    // Only the known evaluator catalog may receive revocations after refused registration.
    for t in &mut revoked.batch.transactions {
        t.changes.retain(|c| {
            matches!(
                c,
                EvidenceChange::Validity {
                    binding_id: 1 | 2,
                    ..
                }
            )
        });
    }
    finish(&mut e, revoked);
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| !b.usable));
}

#[test]
fn oversized_discovery_capacity_is_refused_before_retaining_it() {
    let mut s = state(false);
    let mut descriptor = market("n", 900001);
    descriptor.series_id.reserve(256);
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::RegisterMarket {
                token: s.token(Stream::Metadata),
                descriptor,
                bindings: vec![binding(3, "n", Side::Yes), binding(4, "n", Side::No)],
                capabilities: vec![(3, controlled()), (4, controlled())]
            }
        ))
        .unwrap_err()
        .code,
        "resource_limit"
    );
    assert_eq!(s.snapshot(0).unwrap().markets.len(), 1);
    assert_eq!(s.snapshot(0).unwrap().bindings.len(), 2);
    assert!(matches!(
        s.stop(0).unwrap().batch.current_admission,
        Admission::Blocked("stopped")
    ));
}

#[test]
fn reverse_disjoint_coverage_never_fabricates_contiguous_startup_history() {
    let mut r = serde_json::from_str::<serde_json::Value>(rules().json()).unwrap();
    r["measurements"] = json!([{"id":"average","operation":"rolling_average","of":{"reference":"ask"},"lookback_seconds":2}]);
    let strategy = ValidatedStrategy::parse(&r.to_string()).unwrap();
    let mut s = SourceState::new(
        &strategy,
        &[market("m", 5000)],
        &[binding(1, "m", Side::Yes), binding(2, "m", Side::No)],
        &[(1, controlled()), (2, controlled())],
    )
    .unwrap();
    connect(&mut s, 0);
    initialize(&mut s, false, 0);
    for (start, end, expected) in [(2000, 3000, false), (0, 1000, false), (1000, 2000, true)] {
        s.apply(envelope(
            3000,
            SourceEvent::Update {
                stream: Stream::Book,
                token: s.token(Stream::Book),
                updates: vec![],
                coverage: vec![CoverageFact {
                    binding_id: 1,
                    start_ms: start,
                    end_ms: end,
                }],
            },
        ))
        .unwrap();
        let b = &s.snapshot(3000).unwrap().bindings[0];
        assert_eq!(b.history_ready, expected);
        assert_eq!(b.covered_through_ms, Some(3000));
    }
}

#[test]
fn unready_expired_markets_release_slots_but_preserve_series_identity() {
    let mut s = state(true);
    assert_eq!(
        s.market_ids().collect::<Vec<_>>(),
        s.snapshot(0)
            .unwrap()
            .markets
            .iter()
            .map(|m| m.descriptor.market_id.as_str())
            .collect::<Vec<_>>()
    );
    let old = s.stamp(&[1, 3]).unwrap();
    s.apply(envelope(1800001, SourceEvent::Advance)).unwrap();
    assert!(!s.is_current(&old));
    let view = s.snapshot(1800001).unwrap();
    assert!(view.markets.is_empty() && view.bindings.is_empty());
    assert!(s.market_ids().next().is_none());
    let mut wrong = market("future", 1800001);
    wrong.series_id = "other_series".into();
    assert_eq!(
        s.apply(envelope(
            1800001,
            SourceEvent::RegisterMarket {
                token: s.token(Stream::Metadata),
                descriptor: wrong,
                bindings: vec![],
                capabilities: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "identity_mismatch"
    );
    s.apply(envelope(
        1800001,
        SourceEvent::RegisterMarket {
            token: s.token(Stream::Metadata),
            descriptor: market("future", 1800001),
            bindings: vec![
                binding(1, "future", Side::Yes),
                binding(2, "future", Side::No),
            ],
            capabilities: vec![(1, controlled()), (2, controlled())],
        },
    ))
    .unwrap();
    assert_eq!(s.snapshot(1800001).unwrap().markets.len(), 1);
    assert_eq!(s.market_ids().collect::<Vec<_>>(), vec!["future"]);
    assert!(!s.is_current(&old));
}

#[test]
fn invalid_source_errors_require_delivered_revocation_before_initialization() {
    let complete = |e: &mut Evaluator, effect| finish_with_objective(e, effect, Some(objective()));
    for (bad, code, reason) in [
        ("100.000001", "invalid_value", Unavailable::InvalidValue),
        ("31", "inconsistent_evidence", Unavailable::Inconsistent),
    ] {
        let mut s = state(false);
        let mut e = evaluator();
        finish(&mut e, initialize(&mut s, false, 0));
        verify(&mut s, "m", 0);
        finish(
            &mut e,
            s.apply(envelope(
                1,
                SourceEvent::Boundary {
                    market_id: "m".into(),
                },
            ))
            .unwrap(),
        );
        let observed = values(
            &mut s,
            1,
            vec![update(binding(1, "m", Side::Yes), "30", Some(1), 100)],
        );
        assert!(complete(&mut e, observed).candidate.is_some());
        let old = s.stamp(&[1, 2]).unwrap();
        assert_eq!(
            s.apply(envelope(
                2,
                SourceEvent::Update {
                    stream: Stream::Book,
                    token: s.token(Stream::Book),
                    updates: vec![update(binding(1, "m", Side::Yes), bad, Some(1), 100)],
                    coverage: vec![]
                }
            ))
            .unwrap_err()
            .code,
            code
        );
        assert!(!s.is_current(&old));
        assert_eq!(
            s.apply(envelope(
                2,
                SourceEvent::Update {
                    stream: Stream::Book,
                    token: s.token(Stream::Book),
                    updates: vec![update(binding(1, "m", Side::Yes), "30", None, 100)],
                    coverage: vec![]
                }
            ))
            .unwrap_err()
            .code,
            "invalid_request"
        );
        // The consumer must deliver this explicit revocation before further ordinary intake.
        let revoked = s
            .invalidate(
                Scope::Stream(Stream::Book),
                reason,
                2,
                Some(GapInterval {
                    start_ms: 1,
                    end_ms: 2,
                }),
            )
            .unwrap();
        assert!(complete(&mut e, revoked).candidate.is_none());
        assert!(s.snapshot(2).unwrap().bindings.iter().all(|b| !b.usable));
        let recovered = initialize(&mut s, false, 2);
        assert!(complete(&mut e, recovered).candidate.is_some());
    }
}

#[test]
fn metadata_loss_without_reference_inputs_revokes_market_evidence_until_reverified() {
    for scope in [
        Scope::Stream(Stream::Metadata),
        Scope::Source(Source::Kalshi),
        Scope::Provider(Provider::Kalshi),
        Scope::Market,
        Scope::All,
    ] {
        let mut s = state(false);
        let mut e = evaluator();
        finish(&mut e, initialize(&mut s, false, 0));
        finish(&mut e, verify(&mut s, "m", 0));
        let opening = s
            .apply(envelope(
                1,
                SourceEvent::Boundary {
                    market_id: "m".into(),
                },
            ))
            .unwrap();
        assert!(
            finish_with_objective(&mut e, opening, Some(objective()))
                .candidate
                .is_some()
        );
        let stamp = s.stamp(&[1, 2]).unwrap();
        let metadata = s.token(Stream::Metadata);
        let book = s.token(Stream::Book);
        let loss = s
            .invalidate(scope, Unavailable::MetadataChanged, 2, None)
            .unwrap();
        assert!(!s.is_current(&stamp), "{scope:?}");
        let view = s.snapshot(2).unwrap();
        assert!(!view.markets[0].verified);
        assert_ne!(view.markets[0].preparation, Preparation::Ready);
        assert!(
            finish_with_objective(&mut e, loss, Some(objective()))
                .candidate
                .is_none()
        );
        assert_eq!(
            s.apply(envelope(
                2,
                SourceEvent::Metadata {
                    token: metadata,
                    market_id: "m".into(),
                    verified: true
                }
            ))
            .unwrap_err()
            .code,
            "obsolete_generation"
        );
        assert_eq!(
            s.apply(envelope(
                2,
                SourceEvent::Update {
                    stream: Stream::Book,
                    token: book,
                    updates: vec![update(binding(1, "m", Side::Yes), "30", None, 100)],
                    coverage: vec![]
                }
            ))
            .unwrap_err()
            .code,
            "obsolete_generation"
        );
        // Fresh book data alone cannot restore admission while market metadata is unverified.
        let fresh = initialize(&mut s, false, 2);
        assert!(
            finish_with_objective(&mut e, fresh, Some(objective()))
                .candidate
                .is_none()
        );
        let verified = verify(&mut s, "m", 2);
        assert!(
            finish_with_objective(&mut e, verified, Some(objective()))
                .candidate
                .is_some()
        );
        assert!(
            s.apply(envelope(
                2,
                SourceEvent::Boundary {
                    market_id: "m".into()
                }
            ))
            .unwrap()
            .batch
            .transactions
            .is_empty()
        );
    }
    let mut s = state(false);
    initialize(&mut s, false, 0);
    verify(&mut s, "m", 0);
    let stamp = s.stamp(&[1, 2]).unwrap();
    s.invalidate(Scope::Stream(Stream::Trades), Unavailable::Gap, 0, None)
        .unwrap();
    assert!(s.is_current(&stamp));
    assert!(s.snapshot(0).unwrap().markets[0].verified);
}

#[test]
fn initial_readiness_must_precede_open_even_for_zero_history_or_late_renewal() {
    for (data_at, metadata_at, expired) in
        [(0, 0, false), (1, 0, false), (0, 1, false), (0, 0, true)]
    {
        let mut s = state(false);
        let mut e = evaluator();
        if metadata_at == 0 {
            finish(&mut e, verify(&mut s, "m", 0));
        }
        let until = if expired { 1 } else { 100 };
        let first = values(
            &mut s,
            data_at,
            vec![
                update(binding(1, "m", Side::Yes), "30", None, until),
                update(binding(2, "m", Side::No), "60", None, until),
            ],
        );
        finish(&mut e, first);
        if metadata_at == 1 {
            finish(&mut e, verify(&mut s, "m", 1));
        }
        if expired {
            let renewal = values(
                &mut s,
                1,
                [Side::Yes, Side::No]
                    .into_iter()
                    .enumerate()
                    .map(|(n, side)| BindingUpdate {
                        binding: binding(n as u32 + 1, "m", side),
                        unit: RuleUnit::ContractCents,
                        value: None,
                        valid_until_ms: Some(100),
                    })
                    .collect(),
            );
            finish(&mut e, renewal);
        }
        let boundary = s
            .apply(envelope(
                1,
                SourceEvent::Boundary {
                    market_id: "m".into(),
                },
            ))
            .unwrap();
        let eligible = data_at == 0 && metadata_at == 0 && !expired;
        assert_eq!(
            !boundary.batch.transactions.is_empty(),
            eligible,
            "data={data_at} metadata={metadata_at} expired={expired}"
        );
        assert_eq!(
            finish_with_objective(&mut e, boundary, Some(objective()))
                .candidate
                .is_some(),
            eligible
        );
    }
}

#[test]
fn snapshot_distinguishes_awaiting_values_expired_data_and_explicit_refusal() {
    let mut s = state(false);
    let awaiting = s.snapshot(0).unwrap().bindings[0].reason;
    assert_eq!(awaiting, Some(Unavailable::Awaiting));
    values(
        &mut s,
        0,
        vec![update(binding(1, "m", Side::Yes), "30", None, 1)],
    );
    let expired = s.snapshot(1).unwrap().bindings[0].reason;
    assert_ne!(awaiting, expired);
    s.apply(envelope(
        1,
        SourceEvent::Connection {
            provider: Provider::Kalshi,
            token: s.token(Stream::Book),
            state: Connection::AccessRefused,
        },
    ))
    .unwrap();
    let refused = s.snapshot(1).unwrap().bindings[0].reason;
    assert_eq!(refused, Some(Unavailable::Access));
    assert_ne!(refused, awaiting);
    assert_ne!(refused, expired);
}

#[test]
fn initial_skip_and_retirement_preserve_prepared_successor_readiness() {
    let mut s = state(true);
    values(
        &mut s,
        0,
        vec![
            update(binding(3, "n", Side::Yes), "30", None, 2000000),
            update(binding(4, "n", Side::No), "60", None, 2000000),
        ],
    );
    verify(&mut s, "n", 0);
    assert!(
        s.apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into()
            }
        ))
        .unwrap()
        .batch
        .transactions
        .is_empty()
    );
    initialize(&mut s, true, 2);
    verify(&mut s, "m", 2);
    assert!(
        s.apply(envelope(
            2,
            SourceEvent::Boundary {
                market_id: "m".into()
            }
        ))
        .unwrap()
        .batch
        .transactions
        .is_empty()
    );
    s.apply(envelope(900001, SourceEvent::Advance)).unwrap();
    let opening = s
        .apply(envelope(
            900001,
            SourceEvent::Boundary {
                market_id: "n".into(),
            },
        ))
        .unwrap();
    assert!(
        matches!(opening.batch.transactions[0].changes[0], EvidenceChange::MarketBoundary {ref market_id} if market_id=="n")
    );
    assert_eq!(
        s.snapshot(900001).unwrap().current_market.as_deref(),
        Some("n")
    );
}

#[test]
fn current_window_before_go_displays_without_entry_and_prepares_the_next_full_window() {
    for open in [-60000, -899999] {
        let markets = vec![market("m", open), market("n", open + 900000)];
        let bindings = vec![
            binding(1, "m", Side::Yes),
            binding(2, "m", Side::No),
            binding(3, "n", Side::Yes),
            binding(4, "n", Side::No),
        ];
        let caps: Vec<_> = bindings
            .iter()
            .map(|b| (b.binding_id, controlled()))
            .collect();
        let mut e = Evaluator::new(
            rules(),
            RunContext {
                strategy_version: "v1".into(),
                run_id: "midwindow".into(),
                go_time: "2026-11-15T00:00:00Z".into(),
                series_id: "btc15m".into(),
                markets: markets.clone(),
                bindings: bindings.clone(),
            },
        )
        .unwrap();
        let mut s = SourceState::new(&rules(), &markets, &bindings, &caps).unwrap();
        connect(&mut s, 0);
        let initial = initialize(&mut s, true, 0);
        assert!(matches!(
            initial.batch.current_admission,
            Admission::Blocked("unverified_metadata")
        ));
        finish(&mut e, initial);
        finish(&mut e, verify(&mut s, "m", 0));
        assert_eq!(s.snapshot(0).unwrap().current_market.as_deref(), Some("m"));
        finish(&mut e, verify(&mut s, "n", 0));
        assert_eq!(s.snapshot(0).unwrap().current_market.as_deref(), Some("m"));
        let late = s
            .apply(envelope(
                0,
                SourceEvent::Boundary {
                    market_id: "m".into(),
                },
            ))
            .unwrap();
        assert!(late.batch.transactions.is_empty());
        assert!(
            finish_with_objective(&mut e, late, Some(objective()))
                .candidate
                .is_none()
        );
        assert_eq!(s.snapshot(0).unwrap().current_market.as_deref(), Some("m"));
        let old = s.stamp(&[1]).unwrap();
        let next_open = (open + 900000) as u64;
        let next = s
            .apply(envelope(
                next_open,
                SourceEvent::Boundary {
                    market_id: "n".into(),
                },
            ))
            .unwrap();
        assert!(
            matches!(&next.batch.transactions[0].changes[0],EvidenceChange::MarketBoundary {market_id} if market_id=="n")
        );
        let view = s.snapshot(next_open).unwrap();
        assert_eq!(view.current_market.as_deref(), Some("n"));
        assert_eq!(view.markets.len(), 1);
        assert_eq!(view.bindings.len(), 2);
        assert!(!s.is_current(&old));
        // Exercise evaluator handoff with a 1ms remainder; the 60s-offset case checks source transitions without walking 14 minutes.
        if next_open == 1 {
            let mut objective = objective();
            objective.market_id = "n".into();
            assert_eq!(
                finish_with_objective(&mut e, next, Some(objective))
                    .candidate
                    .unwrap()
                    .market_id,
                "n"
            );
        }
    }
}

#[test]
fn windows_closed_before_go_are_unusable_and_retire_without_unsigned_wrap() {
    let bindings = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
    let mut s = SourceState::new(
        &rules(),
        &[market("m", -900001)],
        &bindings,
        &[(1, controlled()), (2, controlled())],
    )
    .unwrap();
    connect(&mut s, 0);
    initialize(&mut s, false, 0);
    verify(&mut s, "m", 0);
    assert!(s.snapshot(0).unwrap().current_market.is_none());
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| !b.usable));
    s.apply(envelope(0, SourceEvent::Advance)).unwrap();
    let view = s.snapshot(0).unwrap();
    assert!(view.markets.is_empty() && view.bindings.is_empty());
}

#[test]
fn late_metadata_verification_updates_display_without_another_boundary() {
    let mut s = state(false);
    let mut e = evaluator();
    finish(&mut e, initialize(&mut s, false, 0));
    let opening = s
        .apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap();
    assert!(opening.batch.transactions.is_empty());
    assert!(
        finish_with_objective(&mut e, opening, Some(objective()))
            .candidate
            .is_none()
    );
    assert!(s.snapshot(1).unwrap().current_market.is_none());
    finish(&mut e, s.apply(envelope(2, SourceEvent::Advance)).unwrap());
    let before = s.stamp(&[1, 2]).unwrap();
    let late = verify(&mut s, "m", 2);
    assert!(late.batch.transactions.is_empty());
    assert!(
        finish_with_objective(&mut e, late, Some(objective()))
            .candidate
            .is_none()
    );
    let view = s.snapshot(2).unwrap();
    assert!(view.markets[0].verified);
    assert_eq!(view.markets[0].preparation, Preparation::Ready);
    assert_eq!(view.current_market.as_deref(), Some("m"));
    assert!(!s.is_current(&before));
    let duplicate = s
        .apply(envelope(
            2,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap();
    assert!(duplicate.batch.transactions.is_empty());
    assert!(
        finish_with_objective(&mut e, duplicate, Some(objective()))
            .candidate
            .is_none()
    );
}

#[test]
fn metadata_display_does_not_consume_or_repeat_an_eligible_boundary() {
    let mut s = state(false);
    let mut e = evaluator();
    finish(&mut e, initialize(&mut s, false, 0));
    finish(&mut e, verify(&mut s, "m", 0));
    assert!(s.snapshot(0).unwrap().current_market.is_none());
    let display = verify(&mut s, "m", 1);
    assert!(display.batch.transactions.is_empty());
    assert!(
        finish_with_objective(&mut e, display, Some(objective()))
            .candidate
            .is_none()
    );
    assert_eq!(s.snapshot(1).unwrap().current_market.as_deref(), Some("m"));
    let boundary = s
        .apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap();
    assert_eq!(boundary.batch.transactions.len(), 1);
    assert!(
        finish_with_objective(&mut e, boundary, Some(objective()))
            .candidate
            .is_some()
    );
    let duplicate = s
        .apply(envelope(
            1,
            SourceEvent::Boundary {
                market_id: "m".into(),
            },
        ))
        .unwrap();
    assert!(duplicate.batch.transactions.is_empty());
}

#[test]
fn changed_successor_schedule_revokes_then_requires_fresh_metadata_and_data() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "m", 0);
    verify(&mut s, "n", 0);
    let stamp = s.stamp(&[3, 4]).unwrap();
    let old_meta = s.token(Stream::Metadata);
    let old_book = s.token(Stream::Book);
    let changed = market("n", 910001);
    s.apply(envelope(
        0,
        SourceEvent::ReplaceMarket {
            token: old_meta,
            expected: market("n", 900001),
            replacement: changed.clone(),
        },
    ))
    .unwrap();
    assert!(!s.is_current(&stamp));
    let view = s.snapshot(0).unwrap();
    assert_eq!(view.markets.len(), 2);
    assert_eq!(view.markets[1].descriptor, changed);
    assert!(!view.markets[1].verified);
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::Metadata {
                token: old_meta,
                market_id: "n".into(),
                verified: true
            }
        ))
        .unwrap_err()
        .code,
        "obsolete_generation"
    );
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::Update {
                stream: Stream::Book,
                token: old_book,
                updates: vec![update(binding(3, "n", Side::Yes), "31", None, 2000000)],
                coverage: vec![]
            }
        ))
        .unwrap_err()
        .code,
        "obsolete_generation"
    );
    verify(&mut s, "m", 0);
    verify(&mut s, "n", 0);
    assert_ne!(
        s.snapshot(0).unwrap().markets[1].preparation,
        Preparation::Ready
    );
    initialize(&mut s, true, 0);
    assert_eq!(
        s.snapshot(0).unwrap().markets[1].preparation,
        Preparation::Ready
    );
    let boundary = s
        .apply(envelope(
            910001,
            SourceEvent::Boundary {
                market_id: "n".into(),
            },
        ))
        .unwrap();
    assert!(boundary.batch.transactions.iter().any(|t| {
        t.changes
            .iter()
            .any(|c| matches!(c, EvidenceChange::MarketBoundary { .. }))
    }));
    let duplicate = s
        .apply(envelope(
            910001,
            SourceEvent::Boundary {
                market_id: "n".into(),
            },
        ))
        .unwrap();
    assert!(duplicate.batch.transactions.is_empty());
    let mut changed_again = changed.clone();
    changed_again.open_ms += 1;
    changed_again.close_ms += 1;
    s.apply(envelope(
        910001,
        SourceEvent::ReplaceMarket {
            token: s.token(Stream::Metadata),
            expected: changed,
            replacement: changed_again,
        },
    ))
    .unwrap();
    assert!(s.snapshot(910001).unwrap().stopped);
}

#[test]
fn replacement_noop_preserves_readiness_and_bad_schedule_stays_revoked() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "n", 0);
    let stamp = s.stamp(&[3, 4]).unwrap();
    let token = s.token(Stream::Metadata);
    s.apply(envelope(
        0,
        SourceEvent::ReplaceMarket {
            token,
            expected: market("n", 900001),
            replacement: market("n", 900001),
        },
    ))
    .unwrap();
    assert!(s.is_current(&stamp));
    assert_eq!(
        s.snapshot(0).unwrap().markets[1].preparation,
        Preparation::Ready
    );
    assert!(
        s.apply(envelope(
            0,
            SourceEvent::ReplaceMarket {
                token,
                expected: market("n", 900001),
                replacement: market("n", 1)
            }
        ))
        .is_err()
    );
    assert!(!s.is_current(&stamp));
    assert!(s.apply(envelope(0, SourceEvent::Advance)).is_err());
    s.invalidate(Scope::Market, Unavailable::MetadataChanged, 0, None)
        .unwrap();
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| !b.usable));
}

#[test]
fn replacement_expected_descriptor_must_match_and_late_verification_never_enters() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "n", 0);
    let stamp = s.stamp(&[3, 4]).unwrap();
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::ReplaceMarket {
                token: s.token(Stream::Metadata),
                expected: market("n", 900002),
                replacement: market("n", 910001)
            }
        ))
        .unwrap_err()
        .code,
        "identity_mismatch"
    );
    assert!(s.is_current(&stamp));
    let changed = market("n", 910001);
    s.apply(envelope(
        910002,
        SourceEvent::ReplaceMarket {
            token: s.token(Stream::Metadata),
            expected: market("n", 900001),
            replacement: changed.clone(),
        },
    ))
    .unwrap();
    verify(&mut s, "n", 910002);
    values(
        &mut s,
        910002,
        vec![
            update(binding(3, "n", Side::Yes), "31", None, 2000000),
            update(binding(4, "n", Side::No), "61", None, 2000000),
        ],
    );
    let boundary = s
        .apply(envelope(
            910002,
            SourceEvent::Boundary {
                market_id: "n".into(),
            },
        ))
        .unwrap();
    assert!(boundary.batch.transactions.iter().all(|t| {
        t.changes
            .iter()
            .all(|c| !matches!(c, EvidenceChange::MarketBoundary { .. }))
    }));
    assert_eq!(
        s.snapshot(910002).unwrap().current_market.as_deref(),
        Some("n")
    );
}
#[test]
fn oversized_schedule_correction_revokes_and_blocks_until_revocation_is_delivered() {
    let mut s = state(true);
    initialize(&mut s, true, 0);
    verify(&mut s, "n", 0);
    let stamp = s.stamp(&[3, 4]).unwrap();
    let mut invalid = market("n", 910001);
    invalid.contract_id = "x".repeat(129);
    assert_eq!(
        s.apply(envelope(
            0,
            SourceEvent::ReplaceMarket {
                token: s.token(Stream::Metadata),
                expected: market("n", 900001),
                replacement: invalid
            }
        ))
        .unwrap_err()
        .code,
        "resource_limit"
    );
    assert!(!s.is_current(&stamp));
    assert!(s.apply(envelope(0, SourceEvent::Advance)).is_err());
    s.invalidate(Scope::Market, Unavailable::MetadataChanged, 0, None)
        .unwrap();
    assert!(s.snapshot(0).unwrap().bindings.iter().all(|b| !b.usable));
}
#[test]
fn changed_descriptor_stops_attached_pending_evaluator_and_explicit_recovery_starts_empty() {
    let mut s = state(false);
    let mut e = evaluator();
    let effect = initialize(&mut s, false, 0);
    finish(&mut e, effect);
    let stamp = s.stamp(&[1, 2]).unwrap();
    let pending = s.apply(envelope(0, SourceEvent::Advance)).unwrap();
    e.begin(pending.batch).unwrap();
    let changed = market("m", 2);
    let revoked = s
        .apply(envelope(
            0,
            SourceEvent::ReplaceMarket {
                token: s.token(Stream::Metadata),
                expected: market("m", 1),
                replacement: changed.clone(),
            },
        ))
        .unwrap();
    assert!(!s.is_current(&stamp));
    // Pending work cannot admit a second batch; terminal Stop preserves its owned storage.
    let retained = e.allocated_bytes();
    assert!(matches!(
        e.process_slice(SliceControl::Stop, None),
        SliceResult::Stopped
    ));
    assert_eq!(e.allocated_bytes(), retained);
    assert!(matches!(
        e.process_slice(SliceControl::Continue, None),
        SliceResult::Stopped
    ));
    assert!(
        revoked
            .batch
            .transactions
            .iter()
            .any(|t| t.changes.iter().any(|c| matches!(
                c,
                EvidenceChange::Validity {
                    valid_until_ms: 0,
                    ..
                }
            )))
    );
    let bindings = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
    let fresh = SourceState::new(
        &rules(),
        std::slice::from_ref(&changed),
        &bindings,
        &[(1, controlled()), (2, controlled())],
    )
    .unwrap();
    let recovered = Evaluator::new(
        rules(),
        RunContext {
            strategy_version: "fresh".into(),
            run_id: "new-epoch".into(),
            go_time: "new controlled epoch".into(),
            series_id: "btc15m".into(),
            markets: vec![changed],
            bindings,
        },
    )
    .unwrap();
    assert!(!fresh.is_current(&stamp));
    assert!(recovered.snapshot().candidate.is_none());
    assert!(recovered.snapshot().checkpoints.is_empty());
}

#[test]
fn display_catalog_is_fixed_disconnected_and_grants_no_capabilities() {
    let s = SourceState::display().unwrap();
    let v = s.snapshot(0).unwrap();
    assert!(v.markets.is_empty());
    assert_eq!(v.sources.len(), 4);
    assert!(
        v.sources
            .iter()
            .all(|(_, c)| *c == Connection::Disconnected)
    );
    assert_eq!(s.requirements().len(), 8);
    assert_eq!(v.bindings.len(), 3);
    for b in &v.bindings {
        assert!(b.value.is_none() && !b.usable);
        // No lookback is requested; that does not authorize historical coverage.
        assert_eq!(b.capabilities, Capabilities::default());
    }
    assert_eq!(
        s.requirements()
            .iter()
            .filter(|r| r.source == Source::Kalshi && r.instrument == "current_contract")
            .count(),
        3
    );
}

#[test]
fn continuous_coverage_preserves_prior_readiness_without_waiving_new_or_expired_readiness() {
    for mode in ["continuous", "first_at_open", "expired", "disjoint"] {
        let mut document: serde_json::Value = serde_json::from_str(rules().json()).unwrap();
        document["measurements"] = json!([{"id":"average","operation":"rolling_average","of":{"reference":"ask"},"lookback_seconds":1}]);
        let strategy = ValidatedStrategy::parse(&document.to_string()).unwrap();
        let bs = vec![binding(1, "m", Side::Yes), binding(2, "m", Side::No)];
        let markets = vec![market("m", 2001)];
        let mut e = Evaluator::new(
            strategy.clone(),
            RunContext {
                strategy_version: "controlled".into(),
                run_id: "continuous-ready".into(),
                go_time: "controlled".into(),
                series_id: "btc15m".into(),
                markets: markets.clone(),
                bindings: bs.clone(),
            },
        )
        .unwrap();
        let mut s = SourceState::new(
            &strategy,
            &markets,
            &bs,
            &[(1, controlled()), (2, controlled())],
        )
        .unwrap();
        connect(&mut s, 0);
        let until = if mode == "expired" { 2001 } else { 10000 };
        let init = values(
            &mut s,
            0,
            vec![
                update(bs[0].clone(), "30", None, until),
                update(bs[1].clone(), "60", None, until),
            ],
        );
        finish(&mut e, init);
        let metadata = verify(&mut s, "m", 0);
        finish(&mut e, metadata);
        let cover = |s: &mut SourceState, start, end| {
            s.apply(envelope(
                end,
                SourceEvent::Update {
                    stream: Stream::Book,
                    token: s.token(Stream::Book),
                    updates: vec![],
                    coverage: vec![
                        CoverageFact {
                            binding_id: 1,
                            start_ms: start,
                            end_ms: end,
                        },
                        CoverageFact {
                            binding_id: 2,
                            start_ms: start,
                            end_ms: end,
                        },
                    ],
                },
            ))
            .unwrap()
        };
        if mode != "first_at_open" {
            let effect = cover(&mut s, 0, 1000);
            finish(&mut e, effect);
        }
        let start = if mode == "disjoint" {
            1001
        } else if mode == "first_at_open" {
            0
        } else {
            1000
        };
        let effect = cover(&mut s, start, 2001);
        finish(&mut e, effect);
        if mode == "expired" {
            let renewal = values(
                &mut s,
                2001,
                bs.iter()
                    .map(|b| BindingUpdate {
                        binding: b.clone(),
                        unit: RuleUnit::ContractCents,
                        value: None,
                        valid_until_ms: Some(10000),
                    })
                    .collect(),
            );
            finish(&mut e, renewal);
        }
        let boundary = s
            .apply(envelope(
                2001,
                SourceEvent::Boundary {
                    market_id: "m".into(),
                },
            ))
            .unwrap();
        let entered = boundary
            .batch
            .transactions
            .iter()
            .flat_map(|t| &t.changes)
            .any(|c| matches!(c, EvidenceChange::MarketBoundary { .. }));
        assert_eq!(entered, mode == "continuous", "{mode}");
        let decision = finish_with_objective(&mut e, boundary, Some(objective()));
        assert_eq!(decision.candidate.is_some(), mode == "continuous", "{mode}");
        if let Some(candidate) = decision.candidate {
            assert_eq!(candidate.side, Side::Yes);
            assert_eq!(candidate.option_id, "buy");
        }
    }
}
