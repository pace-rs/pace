# `doctor`

The checks of `pace doctor`; `doctor.rs` holds the `Finding` and runs the checks
in order. `directories.rs`, `files.rs` and `device.rs` each hold the checks of
one or two rows of doctor's checks table. Each check reads what it needs, hands
the texts to a check without I/O where there is one, and turns every problem
into a finding. `report.rs` prints the findings as text or as rows of the
schema.

## Files

<!-- files -->

| File                               | What it holds                                                                                                                                                                                                                     |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`device.rs`](device.rs)           | The device and setup checks of `pace doctor`: a device ID that another machine writes with too, and a `pace setup` that waits to be continued.                                                                                    |
| [`directories.rs`](directories.rs) | The directory checks of `pace doctor`: pace can write to the config file's directory, the data and the machine directory, or create them; the machine directory lies outside the data directory; and whether portable mode is on. |
| [`files.rs`](files.rs)             | The file checks of `pace doctor`: the global config file, the catalog's files, the manifest in reach and every event file read.                                                                                                   |
| [`report.rs`](report.rs)           | The findings of `pace doctor` as text, one block per finding and a last line that counts them, and as rows of the schema.                                                                                                         |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`commands`](../commands/), [`config`](../config/), [`dirs`](../dirs/),
  [`origin`](../origin.rs), [`project`](../project.rs), [`render`](../render/),
  [`setup`](../setup/).
- Used by: [`commands`](../commands/).

<!-- /uses -->
