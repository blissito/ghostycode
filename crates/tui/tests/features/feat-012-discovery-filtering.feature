Feature: FEAT-012 Discovery Filtering (Palette And Slash Completion)

  Scenario: AC1 Visible user command appears once in the palette with metadata
    Given a workspace with a visible user command that has a description and usage
    When the command palette is queried
    Then the user command appears exactly once with its description and usage
    And no duplicate entry appears for the same effective token

  Scenario: AC2 Hidden user command is runnable but excluded from the palette
    Given a workspace with a hidden user command
    When the user runs the hidden command directly
    Then the user command executes
    When the command palette is queried
    Then the hidden command is absent from the palette

  Scenario: AC3 Visible user command appears in matching slash completion
    Given a workspace with a visible user command that has a description
    When slash completion is queried with a matching prefix
    Then the user command appears exactly once with its description

  Scenario: AC4 Hidden user command is excluded from slash completion
    Given a workspace with a hidden user command
    When slash completion is queried for its prefix
    Then the hidden command is absent from slash completion

  Scenario: AC5 User canonical shadow suppresses a built-in in both surfaces
    Given a workspace with a visible user command owning a built-in canonical token
    When the command palette and slash completion are queried
    Then only the user-owned discovery entry appears for that token
    And its metadata identifies the user command

  Scenario: AC6 User command shadows a built-in alias without hiding canonical access
    Given a workspace with a visible user command owning a built-in alias token
    When slash completion is queried for the shadowed alias
    Then the user command appears and the aliased built-in does not
    When slash completion is queried for the built-in canonical prefix
    Then the built-in canonical command remains available

  Scenario: AT-010 Alias-aware unification - accepted user alias claims a built-in canonical token
    Given a workspace with a visible user command whose accepted alias equals a built-in canonical token
    When the command palette and slash completion are queried for that token
    Then both surfaces suppress the built-in canonical entry
    And the user command is the only discovery entry for that token
