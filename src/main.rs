use aam::examples::EXAMPLES;
use aam::{evaluate, format_env, parse};
use std::process::ExitCode;

struct Options {
    selection: Selection,
    limit: usize,
    trace: bool,
}

enum Selection {
    Example(String),
    Source(String),
    All,
}

fn options(arguments: Vec<String>) -> Result<Options, String> {
    let mut selection = None;
    let mut limit = 1_000;
    let mut trace = true;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--example" | "--expr" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("{argument} requires a value"))?;
                let next = if argument == "--example" {
                    Selection::Example(value)
                } else {
                    Selection::Source(value)
                };
                if selection.replace(next).is_some() {
                    return Err("choose only one of --example, --expr, or --all".to_owned());
                }
            }
            "--all" => {
                if selection.replace(Selection::All).is_some() {
                    return Err("choose only one of --example, --expr, or --all".to_owned());
                }
            }
            "--limit" => {
                limit = arguments
                    .next()
                    .ok_or("--limit requires a nonnegative integer")?
                    .parse()
                    .map_err(|_| "--limit requires a nonnegative integer")?;
            }
            "--no-trace" => trace = false,
            _ => return Err(format!("unknown option {argument:?}")),
        }
    }
    Ok(Options {
        selection: selection.unwrap_or_else(|| Selection::Example("identity".to_owned())),
        limit,
        trace,
    })
}

fn run(name: &str, source: &str, options: &Options) -> Result<(), String> {
    println!("\n{name}: {source}");
    let expression = parse(source).map_err(|error| format!("parse error: {error}"))?;
    let evaluation = evaluate(expression, options.limit);
    if options.trace {
        for (index, state) in evaluation.states.iter().enumerate() {
            println!("\nState {index}\n  {state}");
            if let Some(rule) = evaluation.rules.get(index) {
                println!("  -- {rule} -->");
            }
        }
    }
    match evaluation.outcome {
        Ok(value) => {
            println!("\nResult code: {}", value.expression());
            println!("Result environment: {}", format_env(&value.env));
            println!("Transitions: {}", evaluation.rules.len());
            Ok(())
        }
        Err(error) => Err(format!("{error} ({} transitions)", evaluation.rules.len())),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments == ["--list"] {
        for example in EXAMPLES {
            println!("{}: {}", example.name, example.explanation);
        }
        return ExitCode::SUCCESS;
    }
    let result = options(arguments).and_then(|options| match &options.selection {
        Selection::Source(source) => run("custom", source, &options),
        Selection::Example(name) => {
            let example = EXAMPLES
                .iter()
                .find(|example| example.name == name)
                .ok_or_else(|| format!("unknown example {name:?}; use --list"))?;
            println!("{}", example.explanation);
            run(example.name, example.source, &options)
        }
        Selection::All => {
            for example in EXAMPLES.iter().filter(|example| example.should_finish) {
                run(example.name, example.source, &options)?;
            }
            Ok(())
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
