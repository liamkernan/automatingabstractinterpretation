//! Concrete left-to-right, call-by-value CEK, matching Figure 1 of the AAM paper.
//! No substitution, addresses, abstract stores, or widening are used here.
pub mod abstract_machine;
pub mod examples;
pub mod syntax;

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
pub use syntax::{Expr, parse};

/// CEK maps names directly to closures. BTreeMap keeps trace order stable.
/// Rc shares immutable closures; Rust pointers here do not constitute a CESK store.
pub type Env = BTreeMap<String, Rc<Closure>>;

/// A lambda's code and its saved lexical environment.
/// Separate parameter/body fields ensure a Closure always represents a lambda.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closure {
    pub parameter: String,
    pub body: Expr,
    pub env: Env,
}

impl Closure {
    pub fn expression(&self) -> Expr {
        Expr::lam(&self.parameter, self.body.clone())
    }
}

/// Explicit pending work: mt, ar(argument, environment, rest), or fn(closure, rest).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kont {
    Mt,
    Ar {
        argument: Expr,
        env: Env,
        rest: Box<Kont>,
    },
    Fn {
        function: Closure,
        rest: Box<Kont>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub control: Expr,
    pub env: Env,
    pub kont: Kont,
}

impl State {
    /// inj(e) = <e, empty environment, empty continuation>.
    pub fn initial(program: Expr) -> Self {
        Self {
            control: program,
            env: Env::new(),
            kont: Kont::Mt,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Lookup,
    EvaluateFunction,
    EvaluateArgument,
    Apply,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::Lookup => "lookup: restore the variable's closure",
            Self::EvaluateFunction => "application: evaluate the function; push ar",
            Self::EvaluateArgument => "function ready: evaluate the argument; replace ar with fn",
            Self::Apply => "argument ready: enter the function body; pop fn",
        };
        write!(f, "{description}")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    UnboundVariable(String),
    StepLimit { limit: usize },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundVariable(name) => write!(f, "unbound variable {name:?}"),
            Self::StepLimit { limit } => write!(
                f,
                "stopped after {limit} transitions; this does not prove divergence"
            ),
        }
    }
}

/// Each call makes one transition, returns a final closure, or reports an error.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Next { rule: Rule, state: State },
    Halted(Closure),
}

pub fn step(state: &State) -> Result<Step, EvalError> {
    match &state.control {
        // Rule 1: restore BOTH parts of the saved closure, including its environment.
        Expr::Var(name) => {
            let closure = state
                .env
                .get(name)
                .ok_or_else(|| EvalError::UnboundVariable(name.clone()))?;
            Ok(Step::Next {
                rule: Rule::Lookup,
                state: State {
                    control: closure.expression(),
                    env: closure.env.clone(),
                    kont: state.kont.clone(),
                },
            })
        }
        // Rule 2: save the argument with ITS environment; evaluate the function first.
        Expr::App(function, argument) => Ok(Step::Next {
            rule: Rule::EvaluateFunction,
            state: State {
                control: function.as_ref().clone(),
                env: state.env.clone(),
                kont: Kont::Ar {
                    argument: argument.as_ref().clone(),
                    env: state.env.clone(),
                    rest: Box::new(state.kont.clone()),
                },
            },
        }),
        // A lambda is a value. K decides whether to return, evaluate an argument, or apply.
        Expr::Lam(parameter, body) => {
            let value = Closure {
                parameter: parameter.clone(),
                body: body.as_ref().clone(),
                env: state.env.clone(),
            };
            match &state.kont {
                Kont::Mt => Ok(Step::Halted(value)),
                // Rule 3: save the function closure; restore the argument's environment.
                Kont::Ar {
                    argument,
                    env,
                    rest,
                } => Ok(Step::Next {
                    rule: Rule::EvaluateArgument,
                    state: State {
                        control: argument.clone(),
                        env: env.clone(),
                        kont: Kont::Fn {
                            function: value,
                            rest: rest.clone(),
                        },
                    },
                }),
                // Rule 4: extend the FUNCTION'S saved environment, never the caller's.
                Kont::Fn { function, rest } => {
                    let mut body_env = function.env.clone();
                    body_env.insert(function.parameter.clone(), Rc::new(value));
                    Ok(Step::Next {
                        rule: Rule::Apply,
                        state: State {
                            control: function.body.clone(),
                            env: body_env,
                            kont: rest.as_ref().clone(),
                        },
                    })
                }
            }
        }
    }
}

/// Trace includes the initial state and the final (or stopped) state.
/// Cloning favors clarity over speed in this first implementation.
#[derive(Debug)]
pub struct Evaluation {
    pub states: Vec<State>,
    pub rules: Vec<Rule>,
    pub outcome: Result<Closure, EvalError>,
}

pub fn evaluate(program: Expr, max_steps: usize) -> Evaluation {
    let mut state = State::initial(program);
    let mut states = vec![state.clone()];
    let mut rules = Vec::new();
    let outcome = loop {
        match step(&state) {
            Ok(Step::Halted(value)) => break Ok(value),
            Ok(Step::Next { rule, state: next }) => {
                if rules.len() == max_steps {
                    break Err(EvalError::StepLimit { limit: max_steps });
                }
                rules.push(rule);
                state = next;
                states.push(state.clone());
            }
            Err(error) => break Err(error),
        }
    };
    Evaluation {
        states,
        rules,
        outcome,
    }
}

/// Readable traces summarize nested closures by code and saved environment KEYS.
/// For complete nested contents, Debug formatting (println!("{state:#?}")) is available.
pub fn format_env(env: &Env) -> String {
    let bindings: Vec<String> = env
        .iter()
        .map(|(name, closure)| {
            let captures: Vec<&str> = closure.env.keys().map(String::as_str).collect();
            format!(
                "{name} ↦ ⟨{}, saved names: [{}]⟩",
                closure.expression(),
                captures.join(", ")
            )
        })
        .collect();
    format!("{{{}}}", bindings.join(", "))
}

impl fmt::Display for Kont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mt => write!(f, "mt"),
            Self::Ar {
                argument,
                env,
                rest,
            } => write!(f, "ar({argument}, {}, {rest})", format_env(env)),
            Self::Fn { function, rest } => write!(
                f,
                "fn(⟨{}, {}⟩, {rest})",
                function.expression(),
                format_env(&function.env)
            ),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "C: {}\n  E: {}\n  K: {}",
            self.control,
            format_env(&self.env),
            self.kont
        )
    }
}
