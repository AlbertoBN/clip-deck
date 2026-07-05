## ADDED Requirements

### Requirement: Long text opens a full, untruncated preview
Selecting the preview action on a clip whose plain-text representation is long SHALL open a preview
showing the complete text, not the truncated form used in the list row.

#### Scenario: Preview shows text beyond the list row's truncation length
- **WHEN** a clip's list row shows a truncated snippet and the user opens its preview
- **THEN** the preview displays the full, untruncated plain-text content

### Requirement: HTML representations are sanitized before rendering in preview
The preview pane SHALL sanitize an HTML representation (stripping executable script and event-handler
attributes) before rendering it, so a captured clip containing malicious HTML cannot execute script in the
UI process.

#### Scenario: A script tag in captured HTML is not executed
- **WHEN** a clip's HTML representation contains a `<script>` tag
- **THEN** the rendered preview does not execute that script

#### Scenario: Benign HTML formatting still renders
- **WHEN** a clip's HTML representation contains only benign formatting tags (e.g. `<b>`, `<a href>`)
- **THEN** the preview renders that formatting rather than stripping it to plain text

### Requirement: Image representations render from their blob path
The preview pane SHALL render an image representation using its `blob_path`-referenced file, showing the
actual captured image rather than a placeholder or the raw path string.

#### Scenario: Selecting an image clip's preview shows the image
- **WHEN** the user opens the preview for a clip whose representation has `mime_family = Image`
- **THEN** the preview renders that image
