# `catalog`

The parts of `catalog.rs`: roles, clients and their dated rates, project
records, money and its currencies, and the rounding of billable time.
`catalog.rs` holds the `Catalog` of roles, clients and project records, and
re-exports these values.

## Files

<!-- files -->

| File                         | What it holds                                                                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| [`client.rs`](client.rs)     | Clients: who gets billed, in which currency, at which dated rates, and how their billable time rounds.                          |
| [`money.rs`](money.rs)       | Money: an amount as a whole number of its currency's minor unit, the amount for a rate and a duration, and totals per currency. |
| [`project.rs`](project.rs)   | Project records: what pace knows of a project beyond its key, such as its client, its role and whether its time is billable.    |
| [`role.rs`](role.rs)         | Roles: the lasting responsibilities that time goes to, such as work or health, and how a role or a project is archived.         |
| [`rounding.rs`](rounding.rs) | Rounding: the client's rule that rounds billable time to a step, per entry or per day and project; no rounding is the default.  |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: no other module.

<!-- /uses -->
