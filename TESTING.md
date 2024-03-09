# Testing strategy

As of now, we are using (or planning to use) the following types of tests, to
ensure the quality of the `pace` project. These tests are run on every pull
request and push to the main branch in our CI/CD pipeline. All tests are run
using GitHub Actions and are mandatory to pass before merging a pull request.

Developers are encouraged to write tests for their code and to write tests for
code that is not yet tested. We are aiming for a high test coverage and a high
quality of tests.

Before a PR the test suite can be run locally using the following command (using
`just`):

```sh
just pr
```

## End-to-end tests

End-to-end tests are being used to test the CLI commands and their interaction
with the `pace_core` library. They are used to ensure that the CLI commands are
working as expected.

## Fuzz tests

Currently no fuzz tests are being used. We should consider using them to test
functions that parse user input or other untrusted input.

## Integration tests

Integration tests are being used to test the service layer of `pace_core`.
`ActivityStore` and `ActivityTracker` are the main components being tested.

We initialize the `ActivityStore` with an `InMemoryStore` and the
`ActivityTracker` with the `ActivityStore`. We then use the `ActivityTracker` to
perform operations on the `ActivityStore` and assert that the operations are
working as expected.

## Journey tests

Journey tests are being used to test that the workflow of important user stories
are working as expected. One example of a journey test is
`test_hold_resume_journey_for_activities_passes`.

## Mutation tests

Currently no mutation tests are being used. We should consider using them to
test the quality of our tests.

## Snapshot tests

Snapshot tests are being used to test the output of the CLI commands (see Visual
Regression Tests). They are used to ensure that the output of the CLI commands
does not change unexpectedly.

## Property-based tests

Currently no property-based tests are being used.

## Unit tests

Unit tests are being used to test the individual components of `pace_core`.
Especially regarding the `Activity` related traits and structs. But also the
`time` module is being tested extensively.

## Visual regression tests

Visual regression tests are being used to test the output of the CLI commands.
We use `insta_cmd` to take snapshots of the output of the CLI commands and
compare them to the expected output. This is used to ensure that the output of
the CLI commands does not change unexpectedly.
