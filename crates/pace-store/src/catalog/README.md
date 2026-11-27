# `catalog`

The readers of the catalog's files in `PACE_HOME`. `roles.rs` reads
`roles.toml`, `clients.rs` reads `clients.toml` with each client's rounding,
rates and project rates, and `projects.rs` reads one record from
`projects/<key>.toml`. `fields.rs` holds what they share: the check for unknown
keys, required keys, and values such as IDs, durations and an archive.
`catalog.rs` beside this folder calls them and hands the records to
`pace_core::Catalog::new`.

## Files

<!-- files -->

| File                         | What it holds                                                                                                                                           |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`clients.rs`](clients.rs)   | The clients file, `clients.toml`: a table per client key with its name, currency, rounding, dated rates and the rates of its projects (restart.md 3.7). |
| [`fields.rs`](fields.rs)     | The fields of the catalog's records: known keys, required keys, and the values they share, such as IDs, durations and an archive.                       |
| [`projects.rs`](projects.rs) | The project records, `projects/<key>.toml`: one file per project with its ID, key, name, client, role, billing, budget, days and archive.               |
| [`roles.rs`](roles.rs)       | The roles file, `roles.toml`: a table per role key with its ID, kind, name, purpose, weekly target and archive.                                         |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`toml_file`](../toml_file.rs).
- Used by: no other module.

<!-- /uses -->
