[![crates.io](https://img.shields.io/crates/d/xdevs_utils.svg)](https://crates.io/crates/xdevs_utils)
[![crates.io](https://img.shields.io/crates/v/xdevs_utils.svg)](https://crates.io/crates/xdevs_utils)


# Utility Tools for the `xDEVS.rs` Simulator

This crate contains a bunch of useful utilities to use in combination with the `xDEVS` simulator.

## DEVS Model Tree Parsers

It allows you to parse a DEVS model described using the DEVS Model Tree (DMT) notation.
This is useful for other utilities to automatically adapt to different models.

**You must enable the `dmt` feature if you want to use this utility.**

## MQTT Handler for Real-Time Simulation

This allows you to easily connect to an MQTT broker to send/receive events during a RT simulation.
This is quite useful when developing Hardware-in-the-Loop (HIL) simulation or Digital Twins (DTs).

**You must enable the `mqtt` feature if you want to use this utility.**

## References

1. R. Cárdenas, P. Arroba, S. Esteban and J. L. Risco-Martín, "[DEVS over MQTT to Enable Distributed Real-Time Simulation](https://ieeexplore.ieee.org/abstract/document/11118639)," 2025 Annual Modeling and Simulation Conference (ANNSIM), Madrid, Spain, 2025, pp. 1-13.
