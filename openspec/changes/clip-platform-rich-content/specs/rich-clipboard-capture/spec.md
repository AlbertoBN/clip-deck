## ADDED Requirements

### Requirement: Reading the clipboard returns every available representation, not only plain text
The X11 backend's `read_current` SHALL return every representation the clipboard currently offers among
plain text, HTML, and image formats (not just the first one found), so a single copy event that offers
both plain text and HTML yields both.

#### Scenario: Copying rich text yields both plain text and HTML representations
- **WHEN** the X11 clipboard offers both a plain-text and an HTML target for the same copy event
- **THEN** `read_current` returns a snapshot containing both representations

#### Scenario: Copying an image yields an image representation
- **WHEN** the X11 clipboard offers an image target (e.g. `image/png`) for a copy event
- **THEN** `read_current` returns a snapshot containing an image representation with that MIME type

### Requirement: Image representations carry pixel dimensions
An image representation produced by capture SHALL include its width and height, matching the PRD's
`clip_representations.width`/`height` columns, computed from the captured image data itself rather than
left absent.

#### Scenario: Captured image representation reports its dimensions
- **WHEN** a 200x100 pixel image is captured
- **THEN** the resulting representation reports `width = 200` and `height = 100`

### Requirement: A representation format the backend cannot decode is skipped, not fatal
Capture SHALL skip a representation whose format it cannot decode (e.g. a malformed image payload) and
still return the other representations available, rather than failing the entire read.

#### Scenario: An undecodable image does not block a valid plain-text representation
- **WHEN** the clipboard offers a valid plain-text target and a malformed image target for the same copy
  event
- **THEN** `read_current` returns a snapshot containing the plain-text representation, with the malformed
  image representation omitted rather than the whole call failing
