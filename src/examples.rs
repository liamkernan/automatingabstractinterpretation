//! Built-in programs exercise the important environment distinctions.
pub struct Example {
    pub name: &'static str,
    pub source: &'static str,
    pub explanation: &'static str,
    pub should_finish: bool,
}

pub const EXAMPLES: &[Example] = &[
    Example {
        name: "identity",
        source: "(λx. x) (λz. z)",
        explanation: "Pass an identity function to another identity function. Result: λz.z.",
        should_finish: true,
    },
    Example {
        name: "capture",
        source: "((λx. λy. x) (λz. z)) (λw. w)",
        explanation: "The returned function remembers x and ignores y. Result: λz.z.",
        should_finish: true,
    },
    Example {
        name: "shadowing",
        source: "((λx. λx. x) (λz. z)) (λw. w)",
        explanation: "The inner x shadows the outer x. Result: λw.w.",
        should_finish: true,
    },
    Example {
        name: "lexical",
        source: "(λx. (λf. (λx. f (λu. u)) (λb. λc. c)) (λy. x)) (λa. a)",
        explanation: "f uses its definition-time x despite a different caller x. Result: λa.a.",
        should_finish: true,
    },
    Example {
        name: "argument-env",
        source: "(λx. ((λx. λy. y) (λb. λc. c)) x) (λa. a)",
        explanation: "Argument x uses its saved caller environment. Result: λa.a.",
        should_finish: true,
    },
    Example {
        name: "open-closure",
        source: "(λx. λy. x) (λz. z)",
        explanation: "Returns code λy.x together with a saved binding for x.",
        should_finish: true,
    },
    Example {
        name: "omega",
        source: "(λx. x x) (λx. x x)",
        explanation: "This mathematical program diverges. The runner stops at its step limit.",
        should_finish: false,
    },
    Example {
        name: "strict",
        source: "(λunused. λz. z) ((λx. x x) (λx. x x))",
        explanation: "Call-by-value evaluates even an unused argument; this also diverges.",
        should_finish: false,
    },
];
