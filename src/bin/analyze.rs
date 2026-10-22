use aam::abstract_machine::{Program, analyze};
use aam::{examples::EXAMPLES, parse};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut source = EXAMPLES[0].source.to_owned();
    let mut selected = false;
    let mut max_states = 50_000;
    let mut json = false;
    let mut args = std::env::args().skip(1);
    while let Some(option) = args.next() {
        match option.as_str() {
            "--expr" | "--example" => {
                if selected {
                    return Err("choose only one source".into());
                }
                selected = true;
                let value = args.next().ok_or("missing source value")?;
                source = if option == "--expr" {
                    value
                } else {
                    EXAMPLES
                        .iter()
                        .find(|e| e.name == value)
                        .ok_or("unknown example")?
                        .source
                        .to_owned()
                };
            }
            "--max-states" => {
                max_states = args
                    .next()
                    .ok_or("missing state budget")?
                    .parse()
                    .map_err(|_| "state budget must be a positive integer")?;
            }
            "--json" => json = true,
            _ => return Err(format!("unknown option {option:?}")),
        }
    }
    let program = Program::compile(&parse(&source)?)?;
    let graph = analyze(&program, max_states)?;
    if json {
        println!("{}", graph.json(&program));
    } else {
        println!(
            "Basic AAM (site-based allocation): {} states, {} edges; fixed point complete",
            graph.states.len(),
            graph.edges.len()
        );
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
