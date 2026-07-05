## ADDED Requirements

### Requirement: MIME types are normalized to a canonical lowercase form
The system SHALL normalize incoming MIME type strings (from clipboard targets/selection types) to a
canonical lowercase `type/subtype` form, stripping parameters (e.g. `charset=utf-8`) before the value is
used for representation typing, hashing keys, or storage.

#### Scenario: Mixed-case MIME type is normalized to lowercase
- **WHEN** `normalize_mime("TEXT/HTML")` is called
- **THEN** it returns `"text/html"`

#### Scenario: MIME parameters are stripped
- **WHEN** `normalize_mime("text/plain; charset=utf-8")` is called
- **THEN** it returns `"text/plain"`

### Requirement: MIME type is classified into a representation family
The system SHALL classify a normalized MIME type into one of the families the rest of the workspace
switches on: `Text`, `Html`, `Image`, or `Other`, so capture/preview/paste code does not each re-implement
MIME sniffing.

#### Scenario: text/plain classifies as Text
- **WHEN** `mime_family("text/plain")` is called
- **THEN** it returns `MimeFamily::Text`

#### Scenario: image/png classifies as Image
- **WHEN** `mime_family("image/png")` is called
- **THEN** it returns `MimeFamily::Image`

#### Scenario: Unrecognized MIME type classifies as Other
- **WHEN** `mime_family("application/x-custom-blob")` is called
- **THEN** it returns `MimeFamily::Other`

### Requirement: Malformed MIME input is rejected rather than silently guessed
MIME normalization SHALL return an error rather than fabricating a best-guess value when given a string
that is not a syntactically valid MIME type (no `/` separator, or an empty type or subtype).

#### Scenario: String without a slash is rejected
- **WHEN** `normalize_mime("not-a-mime-type")` is called
- **THEN** it returns an error and does not return a normalized string
