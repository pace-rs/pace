# `config`

The parts of `config.rs`: `key.rs` lists the settings pace knows, with their
keys, their variables and the values that fit. `values.rs` reads and shows each
kind of value. `config.rs` itself loads the layers.

## Files

<!-- files -->

| File                     | What it holds                                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| [`key.rs`](key.rs)       | The settings pace knows: their keys, their variables and the values that fit.                                       |
| [`values.rs`](values.rs) | Reading and showing the values of settings: time zones, days of the week, counts, sources of holds and directories. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: [`application`](../application.rs), [`clock`](../clock.rs),
  [`commands`](../commands/), [`doctor`](../doctor/),
  [`project`](../project.rs), [`render`](../render/), [`setup`](../setup/).

<!-- /uses -->
