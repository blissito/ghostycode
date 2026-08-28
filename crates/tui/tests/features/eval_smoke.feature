Feature: Eval smoke test (binary load and eval step reporting)

  This is an eval smoke test, not a command-surface verification test.
  AT-004 command-surface evidence uses focused palette, slash-completion,
  and help unit tests. This feature confirms the binary loads and the eval
  harness reports step-level success for a shell command.

  Scenario: Binary loads and reports step-level success via eval
    Given a clean GhostyCode evaluation workspace
    When the evaluation harness runs a shell command
    Then the binary exits without crashing
    And the JSON report contains execution steps
