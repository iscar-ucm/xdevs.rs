use std::env;
use std::time::Instant;
use xdevs::devstone::*;
use xdevs::simulation::*;

/// USAGE:
/// `cargo run <MODEL_TYPE> <WIDTH> <DEPTH>`
/// - `<MODEL_TYPE>` must be `LI`, `HI`, `HO`, or `HOmod`.
/// - `WIDTH` must be equal to or greater than 1.
/// - `DEPTH` must be equal to or greater than 1.
fn main() {
    let args: Vec<String> = env::args().collect();
    let model_type = args
        .get(1)
        .expect("first argument must select the model type")
        .clone()
        .to_lowercase();
    let width = args
        .get(2)
        .expect("second argument must select the width")
        .parse()
        .expect("width could not be parsed");
    let depth = args
        .get(3)
        .expect("third argument must select the depth")
        .parse()
        .expect("depth could not be parsed");

    let int_delay = match args.get(4) {
        Some(arg) => arg.parse().expect("int delay could not be parsed"),
        None => 0,
    };
    let ext_delay = match args.get(5) {
        Some(arg) => arg.parse().expect("ext delay could not be parsed"),
        None => int_delay,
    };

    let start = Instant::now();
    let coupled = match model_type.as_str() {
        "li" => LI::create(width, depth, int_delay, ext_delay),
        "hi" => HI::create(width, depth, int_delay, ext_delay),
        "ho" => HO::create(width, depth, int_delay, ext_delay),
        "homod" => HOmod::create(width, depth, int_delay, ext_delay),
        _ => panic!("unknown DEVStone model type"),
    };
    let duration = start.elapsed();
    println!("Model creation time: {duration:?}");
    let start = Instant::now();
    let mut simulator = RootCoordinator::new(coupled);
    let duration = start.elapsed();
    println!("Simulator creation time: {duration:?}");
    let start = Instant::now();
    simulator.simulate(f64::INFINITY);
    let duration = start.elapsed();
    println!("Simulation time: {duration:?}");
}
