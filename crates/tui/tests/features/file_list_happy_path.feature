Feature: Directory listing acceptance
  Scenario: Happy path lists a workspace directory
    Given an offline GhostyCode evaluation workspace
    When the user asks "list the current directory"
    Then the simulated LLM should call the "File" tool with action "list"
    And the tool output should include:
      | entry     |
      | README.md |
      | notes.txt |
      | src       |
