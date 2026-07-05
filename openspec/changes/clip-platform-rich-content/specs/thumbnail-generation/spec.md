## ADDED Requirements

### Requirement: A thumbnail is generated for every captured image representation
The system SHALL generate a bounded-size thumbnail for every image representation captured, storing it as
its own on-disk file referenced separately from the full-size image's `blob_path`, so list rows and quick
previews never need to decode the full-resolution original.

#### Scenario: Capturing a large image produces a thumbnail smaller than the original
- **WHEN** a 4000x3000 pixel image is captured
- **THEN** a thumbnail file is produced whose dimensions are within the configured maximum bound (e.g. no
  larger than 256px on the long edge)

### Requirement: Thumbnail generation preserves aspect ratio
The generated thumbnail SHALL preserve the original image's aspect ratio rather than stretching or
cropping it to a fixed box.

#### Scenario: A non-square image's thumbnail keeps its proportions
- **WHEN** a 4000x2000 pixel (2:1) image is captured and thumbnailed
- **THEN** the resulting thumbnail's width-to-height ratio is (within rounding) still 2:1

### Requirement: Thumbnail generation failure does not block persisting the original image
The system SHALL still persist the original image representation without a thumbnail, rather than
discarding the captured clip, when thumbnail generation fails for a given image (e.g. an unsupported or
corrupt encoding that nonetheless passed capture).

#### Scenario: A thumbnail-generation failure still results in the clip being captured
- **WHEN** an image is captured and thumbnail generation for it fails
- **THEN** the clip is still persisted with its full image representation, with no thumbnail reference set
