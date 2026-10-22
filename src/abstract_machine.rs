//! Finite store-allocated CESK* and a fresh-address concrete instantiation.
//! Each abstract state carries its own store; allocation is fixed by kind and site.
use crate::Expr;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub type Label = usize;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Var(Label),
    Lam(Label),
    App(Label, Label),
}
#[derive(Clone, Debug)]
pub struct Program {
    pub nodes: Vec<Node>,
}
impl Program {
    /// Preorder occurrence labels. Each variable is resolved to its lexical binder's label.
    pub fn compile(expression: &Expr) -> Result<Self, String> {
        fn visit(
            e: &Expr,
            scope: &BTreeMap<String, Label>,
            out: &mut Vec<Node>,
        ) -> Result<Label, String> {
            let label = out.len();
            out.push(Node::Var(0)); // reserve the occurrence before visiting children
            out[label] = match e {
                Expr::Var(x) => Node::Var(*scope.get(x).ok_or_else(|| {
                    format!("free variable {x:?}; analysis requires a closed program")
                })?),
                Expr::Lam(x, body) => {
                    let mut inner = scope.clone();
                    inner.insert(x.clone(), label);
                    Node::Lam(visit(body, &inner, out)?)
                }
                Expr::App(f, a) => Node::App(visit(f, scope, out)?, visit(a, scope, out)?),
            };
            Ok(label)
        }
        let mut nodes = Vec::new();
        visit(expression, &BTreeMap::new(), &mut nodes)?;
        Ok(Self { nodes })
    }
}

/// kind: 0=halt, 1=binding, 2=argument frame, 3=function frame.
/// serial is zero in the abstract machine and fresh in the concrete machine.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Address {
    pub kind: usize,
    pub site: Label,
    pub serial: usize,
}
impl Address {
    fn halt() -> Self {
        Self {
            kind: 0,
            site: 0,
            serial: 0,
        }
    }
    fn abstracted(&self) -> Self {
        Self {
            serial: 0,
            ..self.clone()
        }
    }
}
pub type Environment = BTreeMap<Label, Address>;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Storable {
    Closure(Label, Environment),
    Mt,
    Ar {
        argument: Label,
        env: Environment,
        rest: Address,
        call: Label,
    },
    Fn {
        function: Label,
        env: Environment,
        rest: Address,
    },
}
pub type Store = BTreeMap<Address, BTreeSet<Storable>>;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Control {
    pub expression: Label,
    pub env: Environment,
    pub kont: Address,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineState {
    pub control: Control,
    pub store: Store,
}

pub fn initial() -> MachineState {
    let halt = Address::halt();
    MachineState {
        control: Control {
            expression: 0,
            env: Environment::new(),
            kont: halt.clone(),
        },
        store: BTreeMap::from([(halt, BTreeSet::from([Storable::Mt]))]),
    }
}
fn allocate(kind: usize, site: Label, store: &Store, fresh: bool) -> Address {
    Address {
        kind,
        site,
        serial: if fresh {
            store.keys().map(|a| a.serial).max().unwrap_or(0) + 1
        } else {
            0
        },
    }
}
fn write(store: &Store, address: Address, value: Storable) -> Store {
    let mut result = store.clone();
    // Weak update: never overwrite another possible value at a reused abstract address.
    result.entry(address).or_default().insert(value);
    result
}
pub fn is_final(program: &Program, state: &MachineState) -> bool {
    matches!(program.nodes[state.control.expression], Node::Lam(_))
        && state
            .store
            .get(&state.control.kont)
            .is_some_and(|values| values.contains(&Storable::Mt))
}

/// Exact successor relation. `fresh=false` is abstract; `fresh=true` allocates unused cells.
/// The abstract allocation policy uses only the kind and syntactic allocation site.
pub fn successors(program: &Program, state: &MachineState, fresh: bool) -> BTreeSet<MachineState> {
    let c = &state.control;
    let mut result = BTreeSet::new();
    match program.nodes[c.expression] {
        Node::Var(binder) => {
            if let Some(address) = c.env.get(&binder) {
                for value in state.store.get(address).into_iter().flatten() {
                    if let Storable::Closure(function, env) = value {
                        result.insert(MachineState {
                            control: Control {
                                expression: *function,
                                env: env.clone(),
                                ..c.clone()
                            },
                            store: state.store.clone(),
                        });
                    }
                }
            }
        }
        Node::App(function, argument) => {
            let address = allocate(2, c.expression, &state.store, fresh);
            let frame = Storable::Ar {
                argument,
                env: c.env.clone(),
                rest: c.kont.clone(),
                call: c.expression,
            };
            result.insert(MachineState {
                control: Control {
                    expression: function,
                    env: c.env.clone(),
                    kont: address.clone(),
                },
                store: write(&state.store, address, frame),
            });
        }
        Node::Lam(_) => {
            for frame in state.store.get(&c.kont).into_iter().flatten() {
                match frame {
                    Storable::Ar {
                        argument,
                        env,
                        rest,
                        call,
                    } => {
                        let address = allocate(3, *call, &state.store, fresh);
                        let function = Storable::Fn {
                            function: c.expression,
                            env: c.env.clone(),
                            rest: rest.clone(),
                        };
                        result.insert(MachineState {
                            control: Control {
                                expression: *argument,
                                env: env.clone(),
                                kont: address.clone(),
                            },
                            store: write(&state.store, address, function),
                        });
                    }
                    Storable::Fn {
                        function,
                        env,
                        rest,
                    } => {
                        let Node::Lam(body) = program.nodes[*function] else {
                            unreachable!("only lambdas become function frames")
                        };
                        let address = allocate(1, *function, &state.store, fresh);
                        let mut body_env = env.clone();
                        body_env.insert(*function, address.clone());
                        result.insert(MachineState {
                            control: Control {
                                expression: body,
                                env: body_env,
                                kont: rest.clone(),
                            },
                            store: write(
                                &state.store,
                                address,
                                Storable::Closure(c.expression, c.env.clone()),
                            ),
                        });
                    }
                    Storable::Mt | Storable::Closure(_, _) => {}
                }
            }
        }
    }
    result
}

pub struct Graph {
    pub states: BTreeSet<MachineState>,
    pub edges: BTreeSet<(MachineState, MachineState)>,
}
pub fn analyze(program: &Program, max_states: usize) -> Result<Graph, String> {
    if max_states == 0 {
        return Err("state budget must be positive".into());
    }
    let start = initial();
    let mut states = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start]);
    let mut edges = BTreeSet::new();
    while let Some(state) = queue.pop_front() {
        for next in successors(program, &state, false) {
            edges.insert((state.clone(), next.clone()));
            if states.insert(next.clone()) {
                if states.len() > max_states {
                    return Err(format!("incomplete analysis: exceeded {max_states} states"));
                }
                queue.push_back(next);
            }
        }
    }
    Ok(Graph { states, edges })
}
/// Fresh-address concrete CESK* trace. `complete=false` means the supplied prefix ended.
pub fn concrete_trace(
    program: &Program,
    steps: usize,
) -> Result<(Vec<MachineState>, bool), String> {
    let mut trace = vec![initial()];
    for _ in 0..steps {
        let state = trace.last().unwrap();
        if is_final(program, state) {
            return Ok((trace, true));
        }
        let next = successors(program, state, true);
        if next.len() != 1 {
            return Err("concrete machine was stuck or nondeterministic".into());
        }
        trace.push(next.into_iter().next().unwrap());
    }
    let complete = is_final(program, trace.last().unwrap());
    Ok((trace, complete))
}
fn abstract_env(env: &Environment) -> Environment {
    env.iter().map(|(x, a)| (*x, a.abstracted())).collect()
}
fn abstract_value(value: &Storable) -> Storable {
    match value {
        Storable::Closure(l, e) => Storable::Closure(*l, abstract_env(e)),
        Storable::Mt => Storable::Mt,
        Storable::Ar {
            argument,
            env,
            rest,
            call,
        } => Storable::Ar {
            argument: *argument,
            env: abstract_env(env),
            rest: rest.abstracted(),
            call: *call,
        },
        Storable::Fn {
            function,
            env,
            rest,
        } => Storable::Fn {
            function: *function,
            env: abstract_env(env),
            rest: rest.abstracted(),
        },
    }
}
pub fn abstract_state(state: &MachineState) -> MachineState {
    let mut store: Store = BTreeMap::new();
    for (a, values) in &state.store {
        store
            .entry(a.abstracted())
            .or_default()
            .extend(values.iter().map(abstract_value));
    }
    MachineState {
        control: Control {
            env: abstract_env(&state.control.env),
            kont: state.control.kont.abstracted(),
            ..state.control.clone()
        },
        store,
    }
}
pub fn store_leq(left: &Store, right: &Store) -> bool {
    left.iter()
        .all(|(a, values)| right.get(a).is_some_and(|other| values.is_subset(other)))
}
pub fn covered(left: &MachineState, right: &MachineState) -> bool {
    left.control == right.control && store_leq(&left.store, &right.store)
}

// Stable, lossless JSON interchange. Only numeric labels/addresses occur in states.
// It records complete structural states, not pretty-printed expressions.
fn array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}
fn addr_json(a: &Address) -> String {
    format!("[{},{},{}]", a.kind, a.site, a.serial)
}
fn env_json(e: &Environment) -> String {
    array(e.iter().map(|(x, a)| format!("[{x},{}]", addr_json(a))))
}
fn value_json(v: &Storable) -> String {
    match v {
        Storable::Closure(l, e) => format!("[0,{l},{}]", env_json(e)),
        Storable::Mt => "[1]".into(),
        Storable::Ar {
            argument,
            env,
            rest,
            call,
        } => format!(
            "[2,{argument},{},{},{call}]",
            env_json(env),
            addr_json(rest)
        ),
        Storable::Fn {
            function,
            env,
            rest,
        } => format!("[3,{function},{},{}]", env_json(env), addr_json(rest)),
    }
}
fn store_json(s: &Store) -> String {
    array(
        s.iter()
            .map(|(a, vs)| format!("[{},{}]", addr_json(a), array(vs.iter().map(value_json)))),
    )
}
fn state_json(s: &MachineState) -> String {
    format!(
        "[{},{},{},{}]",
        s.control.expression,
        env_json(&s.control.env),
        store_json(&s.store),
        addr_json(&s.control.kont)
    )
}
fn nodes_json(p: &Program) -> String {
    array(p.nodes.iter().map(|n| match n {
        Node::Var(b) => format!("[0,{b}]"),
        Node::Lam(b) => format!("[1,{b}]"),
        Node::App(f, a) => format!("[2,{f},{a}]"),
    }))
}
impl Graph {
    pub fn json(&self, p: &Program) -> String {
        let states: Vec<_> = self.states.iter().collect();
        let ids: BTreeMap<_, _> = states.iter().enumerate().map(|(i, s)| (*s, i)).collect();
        let edges = array(
            self.edges
                .iter()
                .map(|(a, b)| format!("[{},{}]", ids[a], ids[b])),
        );
        let finals = array(
            states
                .iter()
                .enumerate()
                .filter(|(_, s)| is_final(p, s))
                .map(|(i, _)| i.to_string()),
        );
        format!(
            "{{\"schema\":2,\"mode\":\"baseline\",\"allocation\":\"site\",\"complete\":true,\"nodes\":{},\"initial\":{},\"states\":{},\"edges\":{edges},\"finals\":{finals}}}",
            nodes_json(p),
            ids[&initial()],
            array(states.iter().map(|s| state_json(s)))
        )
    }
}
