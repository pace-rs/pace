# `catalog`

The parts of `catalog.rs`: money and its currencies, the rounding of billable
time, and roles. `catalog.rs` holds the `Catalog` of roles, clients and project
records, and re-exports these values.

## Files

<!-- files -->

| File                         | What it holds                                                                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| [`money.rs`](money.rs)       | Money: an amount as a whole number of its currency's minor unit, the amount for a rate and a duration, and totals per currency. |
| [`role.rs`](role.rs)         | Roles: the lasting responsibilities that time goes to, such as work or health, and how a role or a project is archived.         |
| [`rounding.rs`](rounding.rs) | Rounding: the client's rule that rounds billable time to a step, per entry or per day and project; no rounding is the default.  |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: no other module.

<!-- /uses -->
