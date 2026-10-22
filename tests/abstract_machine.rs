use aam::abstract_machine::*;
use aam::{Expr, examples::EXAMPLES, parse};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

fn program(source: &str) -> Program {
    Program::compile(&parse(source).unwrap()).unwrap()
}

#[test]
fn concrete_cesk_star_matches_every_cek_state_on_the_terminating_examples() {
    fn labels(e: &Expr, out: &mut Vec<Expr>) {
        out.push(e.clone());
        match e {
            Expr::Lam(_, b) => labels(b, out),
            Expr::App(f, a) => {
                labels(f, out);
                labels(a, out)
            }
            _ => {}
        }
    }
    fn closure(label: usize, env: &Environment, store: &Store, syntax: &[Expr]) -> aam::Closure {
        let Expr::Lam(parameter, body) = &syntax[label] else {
            panic!("not a lambda")
        };
        aam::Closure {
            parameter: parameter.clone(),
            body: body.as_ref().clone(),
            env: environment(env, store, syntax),
        }
    }
    fn environment(env: &Environment, store: &Store, syntax: &[Expr]) -> aam::Env {
        let mut result = aam::Env::new();
        for (binder, address) in env {
            let Expr::Lam(name, _) = &syntax[*binder] else {
                panic!("not a binder")
            };
            let values = &store[address];
            assert_eq!(values.len(), 1);
            let Storable::Closure(label, saved) = values.first().unwrap() else {
                panic!("not a closure")
            };
            result.insert(name.clone(), Rc::new(closure(*label, saved, store, syntax)));
        }
        result
    }
    fn kont(address: &Address, store: &Store, syntax: &[Expr]) -> aam::Kont {
        let values = &store[address];
        assert_eq!(values.len(), 1);
        match values.first().unwrap() {
            Storable::Mt => aam::Kont::Mt,
            Storable::Ar {
                argument,
                env,
                rest,
                ..
            } => aam::Kont::Ar {
                argument: syntax[*argument].clone(),
                env: environment(env, store, syntax),
                rest: Box::new(kont(rest, store, syntax)),
            },
            Storable::Fn {
                function,
                env,
                rest,
            } => aam::Kont::Fn {
                function: closure(*function, env, store, syntax),
                rest: Box::new(kont(rest, store, syntax)),
            },
            _ => panic!("not a continuation"),
        }
    }
    for example in EXAMPLES.iter().filter(|e| e.should_finish) {
        let expression = parse(example.source).unwrap();
        let mut syntax = Vec::new();
        labels(&expression, &mut syntax);
        let concrete = aam::evaluate(expression.clone(), 200);
        let (trace, finished) =
            concrete_trace(&Program::compile(&expression).unwrap(), 200).unwrap();
        assert!(finished);
        assert_eq!(trace.len(), concrete.states.len());
        for (stored, expected) in trace.iter().zip(concrete.states) {
            let actual = aam::State {
                control: syntax[stored.control.expression].clone(),
                env: environment(&stored.control.env, &stored.store, &syntax),
                kont: kont(&stored.control.kont, &stored.store, &syntax),
            };
            assert_eq!(actual, expected, "example {}", example.name);
        }
    }
}

#[test]
fn finite_analysis_of_omega_reaches_a_fixed_point() {
    let p = program("(λx.x x) (λx.x x)");
    let graph = analyze(&p, 10000).unwrap();
    assert!(graph.states.len() > 1);
    assert!(graph.states.iter().all(|s| !is_final(&p, s)));
    for state in &graph.states {
        for next in successors(&p, state, false) {
            assert!(graph.edges.contains(&(state.clone(), next)));
        }
    }
}

#[test]
fn repeated_calls_join_values_and_make_the_machine_nondeterministic() {
    let p = program("(λf. (λignored. f (λb. λc.c)) (f (λa.a))) (λx.x)");
    let graph = analyze(&p, 10000).unwrap();
    assert!(graph.states.iter().any(|s| {
        s.store
            .iter()
            .any(|(a, values)| a.kind == 1 && values.len() > 1)
    }));
    assert!(
        graph
            .states
            .iter()
            .any(|s| successors(&p, s, false).len() > 1)
    );
}

#[test]
fn finite_concrete_prefixes_are_locally_simulated_by_the_basic_analyzer() {
    for example in EXAMPLES {
        let p = program(example.source);
        let graph = analyze(&p, 10000).unwrap();
        let (concrete, _) = concrete_trace(&p, 40).unwrap();
        for state in &concrete {
            let abstracted = abstract_state(state);
            assert!(
                graph.states.iter().any(|s| covered(&abstracted, s)),
                "{}",
                example.name
            );
        }
        for pair in concrete.windows(2) {
            let before = abstract_state(&pair[0]);
            let after = abstract_state(&pair[1]);
            for state in graph.states.iter().filter(|s| covered(&before, s)) {
                assert!(
                    successors(&p, state, false)
                        .iter()
                        .any(|s| covered(&after, s))
                );
            }
        }
    }
}

#[test]
fn budgets_never_return_a_partial_graph_as_complete() {
    let p = program("(λx.x) (λz.z)");
    assert!(analyze(&p, 0).is_err());
    assert!(analyze(&p, 1).is_err());
}

#[test]
fn labels_resolve_shadowing_and_reject_free_variables() {
    let p = program("λx.λx.x");
    assert_eq!(p.nodes, vec![Node::Lam(1), Node::Lam(2), Node::Var(1)]);
    assert!(Program::compile(&parse("λx.y").unwrap()).is_err());
}

#[test]
fn abstract_join_preserves_two_concrete_cells_at_one_address() {
    let mut state = initial();
    let a = Address {
        kind: 1,
        site: 0,
        serial: 1,
    };
    let b = Address {
        serial: 2,
        ..a.clone()
    };
    state
        .store
        .insert(a, BTreeSet::from([Storable::Closure(1, BTreeMap::new())]));
    state
        .store
        .insert(b, BTreeSet::from([Storable::Closure(2, BTreeMap::new())]));
    let abstracted = abstract_state(&state);
    assert_eq!(
        abstracted.store.values().map(BTreeSet::len).sum::<usize>(),
        3
    );
    assert_eq!(abstracted.store.len(), 2);
}
