## ADDED Requirements

### Requirement: Simulated paste delivers content to the previously focused window
`simulate_paste` SHALL deliver the given clip content to the window captured as "previously focused" (see
`focused-window-detection`), by placing the content on the clipboard and synthesizing the platform paste
key combination targeted at that window.

#### Scenario: Paste places content on the clipboard before synthesizing the key combination
- **WHEN** `simulate_paste` is called with plain-text content `"hello"`
- **THEN** the clipboard holds `"hello"` at the point the paste key combination is synthesized

### Requirement: Plain-text paste mode strips down to plain text before pasting
When invoked with `PasteMode::PlainText`, `simulate_paste` SHALL use only the clip's plain-text
representation, discarding any richer representation, even if the clip has other representations
available.

#### Scenario: Plain-text mode ignores a non-plain-text representation
- **WHEN** `simulate_paste` is called with `PasteMode::PlainText` on a clip whose only representation is
  non-plain-text
- **THEN** the pasted content is the plain-text-rendered form of that representation, not the raw
  original representation

### Requirement: Paste failure is reported rather than silently no-oping
`simulate_paste` SHALL return an error rather than silently doing nothing when no previously-focused
window is available to target, or when the synthetic key event cannot be delivered.

#### Scenario: No previously-focused window yields an error
- **WHEN** `simulate_paste` is called with no previously-focused window captured
- **THEN** it returns an error rather than succeeding with no observable effect
