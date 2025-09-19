[![crates.io](https://img.shields.io/crates/d/xdevs.svg)](https://crates.io/crates/xdevs)
[![crates.io](https://img.shields.io/crates/v/xdevs.svg)](https://crates.io/crates/xdevs)

# `xDEVS.rs`

Version of the xDEVS simulator for Rust projects.
It allows you to build and simulate computational models following the DEVS formalism.
Its API is easy to use for DEVS practitioners. Currently, their main features are speed and parallelism.

## Blazingly fast 🚀

The Rust version of xDEVS is one of the fastests APIs currently available.
Check the [references](#references) section for more information.

## Unsafe but sound 🔐

We all love purely safe Rust crates. However, it is extremely difficult to provide a safe **AND**
fast implementation of DEVS event propagations. Thus, we decided to use `unsafe` Rust to model
DEVS ports and message propagation. In this way, we can provide the fastest DEVS implementation
we could think of. But don't worry! We have carefully studied the DEVS simulation algorithm
to come up with all the invariants that make our implementation safe even with `unsafe` code.
All the `unsafe` methods of ports come with proper documentation to let you know when it is safe
to use these `unsafe` methods!

**Spoiler alert:** if you don't try to *hack* the DEVS simulation workflow,
then you will always fufill the invariants to safely build your models.

## Fully configurable parallelism 🧶

We rely on the [`rayon`](https://github.com/rayon-rs/rayon) crate to provide parallelism for your simulations.
By default, all the simulation process is done sequentially. However, you can activate different features to
select where you want to take advantage of parallelism:

- `par_start`: it runs in parallel the start methods of your model before starting to simulate.
- `par_collection`: it executes the lambdas of your models in parallel.
- `par_couplings`: it propagates the couplings in parallel.
- `par_transition`: it executes the deltas of your models in parallel.
- `par_stop`: it runs in parallel the stop methods of your model after the simulation.

### Useful combined features

We provide additional features to select handy combinations of features:

- `par_all_no_couplings`: alias for `par_start`, `par_collection`, `par_transition`, and `par_stop` (**THIS IS OUR FAVOURITE**).
- `par_all`: alias for `par_all_no_couplings` and `par_couplings`.

## Real-Time (RT) simulation 🗼

You can run your simulations in real-time!
We rely on the [`tokio`](https://tokio.rs) crate to provide an asynchronous RT simulator.
This feature is specially useful for Hardware-in-the-Loop (HIL) simulation, Digital Twins (DTs),
and hybrid environments where simulation and real hardware coexist.

## References

1. R. Cárdenas, P. Arroba, and J. L. Risco-Martín, "[Lock-Free Simulation Algorithm to Enhance the Performance of Sequential and Parallel DEVS Simulators in Shared-Memory Architectures](https://doi.org/10.1016/j.jpdc.2025.105105)," Journal of Parallel and Distributed Computing (203) pp. 105105, 2025.
2. R. Cárdenas, P. Arroba, S. Esteban and J. L. Risco-Martín, "[DEVS over MQTT to Enable Distributed Real-Time Simulation](https://ieeexplore.ieee.org/abstract/document/11118639)," 2025 Annual Modeling and Simulation Conference (ANNSIM), Madrid, Spain, 2025, pp. 1-13.
3. R. Cárdenas, P. Arroba and J. L. Risco-Martín, "[A New Family of XDEVS Simulators for Enhanced Performance](https://ieeexplore.ieee.org/document/10155396)," 2023 Annual Modeling and Simulation Conference (ANNSIM), Hamilton, ON, Canada, 2023, pp. 668-679.
