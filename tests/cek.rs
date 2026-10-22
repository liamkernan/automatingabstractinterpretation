use aam::examples::EXAMPLES;
use aam::{Closure, EvalError, Expr, Kont, Rule, State, Step, evaluate, parse, step};

fn result(source: &str) -> Closure {
    evaluate(parse(source).unwrap(), 200).outcome.unwrap()
}

#[test]
fn hand_built_identity_runs_through_all_four_rules() {
    // This path does not depend on the parser.
    let program = Expr::app(
        Expr::lam("x", Expr::var("x")),
        Expr::lam("z", Expr::var("z")),
    );
    let evaluation = evaluate(program, 4);
    assert_eq!(
        evaluation.outcome.unwrap().expression(),
        Expr::lam("z", Expr::var("z"))
    );
    assert_eq!(
        evaluation.rules,
        vec![
            Rule::EvaluateFunction,
            Rule::EvaluateArgument,
            Rule::Apply,
            Rule::Lookup
        ]
    );
    assert_eq!(evaluation.states.len(), 5);
    assert!(matches!(evaluation.states.last().unwrap().kont, Kont::Mt));
}

#[test]
fn captures_an_outer_binding_after_the_outer_call_returns() {
    assert_eq!(
        result("((λx. λy. x) (λz. z)) (λw. w)").expression(),
        parse("λz.z").unwrap()
    );
}

#[test]
fn final_answer_keeps_the_environment_needed_by_its_free_variables() {
    let closure = result("(λx. λy. x) (λz. z)");
    assert_eq!(closure.expression(), parse("λy.x").unwrap());
    assert_eq!(closure.env["x"].expression(), parse("λz.z").unwrap());
    assert!(closure.env["x"].env.is_empty());
}

#[test]
fn inner_parameter_shadows_outer_parameter() {
    assert_eq!(
        result("((λx. λx. x) (λz. z)) (λw. w)").expression(),
        parse("λw.w").unwrap()
    );
}

#[test]
fn uses_definition_environment_not_callers_shadowing_binding() {
    let source = EXAMPLES
        .iter()
        .find(|example| example.name == "lexical")
        .unwrap()
        .source;
    assert_eq!(result(source).expression(), parse("λa.a").unwrap());
}

#[test]
fn restores_argument_environment_after_evaluating_the_function() {
    let source = EXAMPLES
        .iter()
        .find(|example| example.name == "argument-env")
        .unwrap()
        .source;
    assert_eq!(result(source).expression(), parse("λa.a").unwrap());
}

#[test]
fn lookup_restores_the_values_own_environment() {
    let closure = result("(λx. x) (λz. z)");
    // The final function was defined in the empty environment, not under x.
    assert!(closure.env.is_empty());
}

#[test]
fn lambda_body_is_not_evaluated_until_called() {
    let closure = result("λx. missing");
    assert_eq!(closure.expression(), parse("λx.missing").unwrap());
}

#[test]
fn unbound_variable_is_an_error_not_a_final_answer() {
    let state = State::initial(Expr::var("missing"));
    assert_eq!(
        step(&state),
        Err(EvalError::UnboundVariable("missing".to_owned()))
    );
}

#[test]
fn evaluates_function_before_argument() {
    let evaluation = evaluate(parse("missing_function missing_argument").unwrap(), 100);
    assert_eq!(
        evaluation.outcome,
        Err(EvalError::UnboundVariable("missing_function".to_owned()))
    );
}

#[test]
fn evaluates_unused_argument_before_entering_function_body() {
    let evaluation = evaluate(parse("(λignored. λz.z) missing").unwrap(), 100);
    assert_eq!(
        evaluation.outcome,
        Err(EvalError::UnboundVariable("missing".to_owned()))
    );
    assert!(!evaluation.rules.contains(&Rule::Apply));
}

#[test]
fn omega_and_an_unused_omega_argument_both_exhaust_fuel() {
    for name in ["omega", "strict"] {
        let source = EXAMPLES
            .iter()
            .find(|example| example.name == name)
            .unwrap()
            .source;
        let evaluation = evaluate(parse(source).unwrap(), 30);
        assert_eq!(evaluation.outcome, Err(EvalError::StepLimit { limit: 30 }));
        assert_eq!(evaluation.rules.len(), 30);
        assert_eq!(evaluation.states.len(), 31);
    }
}

#[test]
fn zero_budget_still_accepts_an_already_final_lambda() {
    let program = parse("λx.x").unwrap();
    assert!(matches!(
        step(&State::initial(program.clone())),
        Ok(Step::Halted(_))
    ));
    let evaluation = evaluate(program, 0);
    assert!(evaluation.outcome.is_ok());
    assert!(evaluation.rules.is_empty());
}

#[test]
fn exact_transition_budget_can_finish_without_an_off_by_one_error() {
    let program = parse("(λx.x) (λz.z)").unwrap();
    assert_eq!(
        evaluate(program.clone(), 3).outcome,
        Err(EvalError::StepLimit { limit: 3 })
    );
    assert!(evaluate(program, 4).outcome.is_ok());
}

#[test]
fn trace_is_a_chain_of_real_machine_transitions() {
    for example in EXAMPLES.iter().filter(|example| example.should_finish) {
        let evaluation = evaluate(parse(example.source).unwrap(), 200);
        assert!(evaluation.outcome.is_ok(), "{}", example.name);
        assert_eq!(evaluation.states.len(), evaluation.rules.len() + 1);
        for (index, pair) in evaluation.states.windows(2).enumerate() {
            assert_eq!(
                step(&pair[0]),
                Ok(Step::Next {
                    rule: evaluation.rules[index],
                    state: pair[1].clone()
                })
            );
        }
        assert!(matches!(
            step(evaluation.states.last().unwrap()),
            Ok(Step::Halted(_))
        ));
    }
}

#[test]
fn parser_supports_both_lambda_spellings_and_standard_precedence() {
    assert_eq!(parse("\\x. x"), parse("λx.x"));
    assert_eq!(
        parse("f x y").unwrap(),
        Expr::app(Expr::app(Expr::var("f"), Expr::var("x")), Expr::var("y"))
    );
    assert_eq!(
        parse("λx. λy. x y").unwrap(),
        Expr::lam(
            "x",
            Expr::lam("y", Expr::app(Expr::var("x"), Expr::var("y")))
        )
    );
}

#[test]
fn printed_programs_parse_back_to_the_same_syntax() {
    for example in EXAMPLES {
        let expression = parse(example.source).unwrap();
        assert_eq!(parse(&expression.to_string()).unwrap(), expression);
    }
}

#[test]
fn malformed_syntax_returns_errors() {
    for source in [
        "", "λ.x", "λx x", "λx.", "(λx.x", "x)", "()", "7", "x + y", "λx.x)",
    ] {
        assert!(parse(source).is_err(), "unexpectedly accepted {source:?}");
    }
}
