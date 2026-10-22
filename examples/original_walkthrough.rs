#[derive(Debug)]
enum Expr {
    Var(String),
    Lam(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
}

fn describe(expr: &Expr) {
    match expr {
        Expr::Var(name) => {
            println!("A variable named {name}");
        }
        Expr::Lam(parameter, body) => {
            println!("A function with parameter {parameter}");
            describe(body);
        }
        Expr::App(function, argument) => {
            println!("A function application");
            describe(function);
            describe(argument);
        }
    }
}

fn main() {
    // Represent λx.x
    let identity = Expr::Lam(String::from("x"), Box::new(Expr::Var(String::from("x"))));

    // Represent λz.z
    let argument = Expr::Lam(String::from("z"), Box::new(Expr::Var(String::from("z"))));

    // Represent (λx.x) (λz.z)
    let program = Expr::App(Box::new(identity), Box::new(argument));

    println!("{:#?}", program);

    describe(&program);

    let example = Expr::Var(String::from("a"));
    describe(&example);
}
