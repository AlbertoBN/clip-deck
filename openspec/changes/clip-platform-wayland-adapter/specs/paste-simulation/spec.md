## MODIFIED Requirements

### Requirement: Paste failure is reported rather than silently no-oping
`simulate_paste` SHALL return an error rather than silently doing nothing when the synthetic key event
cannot be delivered, or when no previously-focused window is available to target on a backend that reports
focus-detection as supported (i.e. the window was expected to be captured but unexpectedly wasn't).

On a backend that reports focus-detection as unsupported (e.g. Wayland compositors that don't expose
focused-window information), `simulate_paste` SHALL NOT treat the absence of a captured window as an error;
instead it SHALL place the clip's content on the clipboard (the paste-key synthesis step is skipped or
best-effort) so the user can complete the paste manually, matching the PRD's requirement to degrade
gracefully rather than fail outright when compositor capabilities are limited.

#### Scenario: No previously-focused window yields an error when focus-detection is supported
- **WHEN** `simulate_paste` is called with no previously-focused window captured on a backend reporting
  focus-detection as supported
- **THEN** it returns an error rather than succeeding with no observable effect

#### Scenario: Missing focus capture degrades to clipboard-only when focus-detection is unsupported
- **WHEN** `simulate_paste` is called on a backend reporting focus-detection as unsupported, with no
  previously-focused window captured
- **THEN** it succeeds, and the clip's content is placed on the clipboard
