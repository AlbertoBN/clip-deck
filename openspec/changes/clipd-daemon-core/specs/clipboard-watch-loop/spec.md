## ADDED Requirements

### Requirement: Every backend capture event is forwarded to ingest in order
The watch loop SHALL forward each `ClipboardBackend` capture event to the ingest pipeline in the order it
was received, without dropping events under normal operation.

#### Scenario: Two sequential capture events are both forwarded
- **WHEN** the backend emits a capture event for `"first"` followed by a capture event for `"second"`
- **THEN** ingest is invoked first with `"first"` and then with `"second"`, in that order

### Requirement: A single ingest failure does not stop the watch loop
If ingest returns an error for one captured event (e.g. a transient store error), the watch loop SHALL log
the failure and continue processing subsequent capture events rather than terminating.

#### Scenario: Watch loop survives one ingest failure
- **WHEN** ingest returns an error for the first of two capture events
- **THEN** the second capture event is still forwarded to ingest and processed normally

### Requirement: Watch loop does not ingest while capture is paused
While capture is paused (via the `PauseCapture` command), the watch loop SHALL NOT forward backend capture
events to ingest, and SHALL resume forwarding immediately once capture is unpaused.

#### Scenario: Capture events are ignored while paused
- **WHEN** capture is paused and the backend emits a capture event
- **THEN** ingest is not invoked for that event

#### Scenario: Capture resumes forwarding once unpaused
- **WHEN** capture is paused, then unpaused, and the backend emits a capture event afterward
- **THEN** ingest is invoked for that event
