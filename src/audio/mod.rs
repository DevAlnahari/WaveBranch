/// Audio module — WAV file parsing, raw PCM sample extraction, and diffing.
///
/// This module is the boundary between the filesystem (arbitrary binary
/// data) and WaveBranch's content-addressable store (pure audio signal).
/// By stripping headers and metadata here, every downstream consumer
/// operates on semantically meaningful audio data only.
pub mod reader;
pub mod diff;
pub mod merge;
