## ADDED Requirements

### Requirement: Capture, ingest, and command handling emit structured log events
The daemon SHALL emit a structured (`tracing`) log event for each capture, each ingest decision (persisted,
excluded, or deduped), and each handled IPC command, each carrying enough structured fields (e.g. clip id,
command name) to correlate related log lines without string-parsing free text.

#### Scenario: An ingested clip's log event carries its clip id
- **WHEN** a clip is successfully ingested
- **THEN** a log event is emitted whose structured fields include that clip's id

#### Scenario: An excluded capture's log event indicates exclusion, not silence
- **WHEN** a capture is skipped due to a matching exclusion rule
- **THEN** a log event is emitted indicating the capture was excluded (not merely absent from the logs)

### Requirement: Log verbosity is filterable without a code change
The daemon SHALL support filtering log verbosity via an environment variable (`RUST_LOG` /
`tracing-subscriber` `EnvFilter`), so a developer or support session can raise verbosity without
recompiling.

#### Scenario: Debug-level filter surfaces debug-level events
- **WHEN** the daemon is started with a filter directive enabling debug-level logs for the ingest module
- **THEN** debug-level ingest log events are emitted, where they were suppressed at the default level
