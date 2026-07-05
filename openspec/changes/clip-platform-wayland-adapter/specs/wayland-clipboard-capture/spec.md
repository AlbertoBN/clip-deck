## ADDED Requirements

### Requirement: Wayland backend reads and writes clipboard content where the protocol is supported
On a compositor supporting the data-control protocol, the Wayland `ClipboardBackend` SHALL support reading
the current clipboard's text content via `read_current` and writing content via `set_current`, matching
the same `ClipboardSnapshot` shape the X11 adapter produces.

#### Scenario: Reading a populated clipboard returns its text
- **WHEN** the Wayland clipboard holds the text `"ssh user@host"` and the compositor supports data-control
- **THEN** `read_current` returns a snapshot whose plain-text representation is `"ssh user@host"`

#### Scenario: Written content is observable on next read
- **WHEN** `set_current` is called with plain-text content `"paste me"` on a supporting compositor
- **THEN** a subsequent `read_current` call returns a snapshot containing `"paste me"`

### Requirement: Watch loop emits a capture event only when content actually changes
Matching the X11 adapter's behavior, the Wayland backend's `start` watch loop SHALL emit a capture event
only when the clipboard's content hash differs from the previously observed hash.

#### Scenario: Copying new content emits one event
- **WHEN** the watch loop is running on a supporting compositor and clipboard content changes from empty
  to `"first copy"`
- **THEN** exactly one capture event is emitted with content `"first copy"`

### Requirement: Backend construction fails clearly when data-control is unavailable
Constructing the Wayland backend SHALL return a clear "unsupported on this compositor" error at startup,
rather than constructing a backend that silently never captures anything, on a compositor that does not
support the data-control protocol at all.

#### Scenario: Unsupported compositor reports a construction error
- **WHEN** the Wayland backend is constructed against a compositor with no data-control support
- **THEN** construction returns an error identifying the missing protocol support, rather than succeeding
  with a backend that will never observe clipboard changes
