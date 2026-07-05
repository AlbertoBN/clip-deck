## ADDED Requirements

### Requirement: Retention pruning runs on a recurring schedule
The daemon SHALL invoke `clip-store`'s retention pruning on a recurring interval (not only at startup),
so clips that age past the configured retention window are eventually pruned without requiring a daemon
restart.

#### Scenario: Prune runs again after the configured interval elapses
- **WHEN** the job scheduler's clock is advanced past one retention-check interval
- **THEN** `clip-store`'s prune is invoked at least once more

### Requirement: A failing background job is logged and does not crash the daemon
If a scheduled job (e.g. retention pruning) returns an error, the daemon SHALL log the failure and
continue running subsequent scheduled runs, rather than panicking or exiting.

#### Scenario: One failed prune run does not stop later runs
- **WHEN** a scheduled prune run returns an error
- **THEN** the next scheduled prune run still executes at its normal interval

### Requirement: No configured retention window means the job is a no-op
When no retention window is configured, the scheduled job SHALL still run on schedule but SHALL perform no
deletions, matching `clip-store`'s no-window-is-a-no-op behavior.

#### Scenario: Scheduled run with no retention window deletes nothing
- **WHEN** no retention window is configured and the scheduled job fires
- **THEN** no clips are removed as a result of that run
