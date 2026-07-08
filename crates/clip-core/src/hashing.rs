//! Stable content hashing for dedupe (blake3).

/// Computes a deterministic, storable content hash for the given bytes.
pub fn hash_content(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Combines a content hash with a normalized MIME type into a dedup key.
pub fn dedup_key(hash: &str, mime: &str) -> String {
    format!("{hash}:{mime}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_same_bytes_twice_yields_same_hash() {
        assert_eq!(hash_content(b"hello world"), hash_content(b"hello world"));
    }

    #[test]
    fn hashing_different_bytes_yields_different_hashes() {
        assert_ne!(hash_content(b"hello"), hash_content(b"world"));
    }

    #[test]
    fn hash_string_round_trips_through_serde() {
        let hash = hash_content(b"hello world");
        let json = serde_json::to_string(&hash).unwrap();
        let round_tripped: String = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, round_tripped);
    }

    #[test]
    fn hash_string_has_fixed_length_regardless_of_input_size() {
        let small = hash_content(b"abc");
        let large = hash_content(&vec![0u8; 300_000]);
        assert_eq!(small.len(), large.len());
    }

    #[test]
    fn same_bytes_different_mime_produce_different_dedup_keys() {
        let bytes = b"same content";
        let key_text = dedup_key(&hash_content(bytes), "text/plain");
        let key_html = dedup_key(&hash_content(bytes), "text/html");
        assert_ne!(key_text, key_html);
    }
}
