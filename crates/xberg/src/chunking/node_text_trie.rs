//! Substring-containment lookup for many node texts against many chunk contents.
//!
//! Checking every node of a page against every chunk on it with `str::contains` is
//! `chunks * nodes` work, which is quadratic when a page holds most of a large document. A trie
//! over the node texts answers "which texts occur in this chunk" in one pass over the chunk.

use ahash::AHashMap;

/// A byte trie from patterns to the ids of the nodes that carry them.
pub(crate) struct NodeTextTrie {
    edges: AHashMap<u64, u32>,
    terminals: AHashMap<u32, Vec<u32>>,
    first_bytes: Box<[bool; 256]>,
    node_count: u32,
}

fn edge_key(node: u32, byte: u8) -> u64 {
    (u64::from(node) << 8) | u64::from(byte)
}

impl NodeTextTrie {
    pub(crate) fn new() -> Self {
        Self {
            edges: AHashMap::new(),
            terminals: AHashMap::new(),
            first_bytes: Box::new([false; 256]),
            node_count: 1,
        }
    }

    /// Registers `pattern` as belonging to `id`. Empty patterns are ignored.
    pub(crate) fn insert(&mut self, pattern: &[u8], id: u32) {
        let Some(&first) = pattern.first() else { return };
        self.first_bytes[usize::from(first)] = true;
        let mut node = 0;
        for &byte in pattern {
            node = *self.edges.entry(edge_key(node, byte)).or_insert_with(|| {
                let created = self.node_count;
                self.node_count += 1;
                created
            });
        }
        self.terminals.entry(node).or_default().push(id);
    }

    /// Sorted, deduplicated ids whose pattern occurs somewhere in `haystack`.
    pub(crate) fn matching_ids(&self, haystack: &[u8]) -> Vec<u32> {
        let mut found = Vec::new();
        for start in 0..haystack.len() {
            if !self.first_bytes[usize::from(haystack[start])] {
                continue;
            }
            let mut node = 0;
            for &byte in &haystack[start..] {
                let Some(&next) = self.edges.get(&edge_key(node, byte)) else {
                    break;
                };
                node = next;
                if let Some(ids) = self.terminals.get(&node) {
                    found.extend_from_slice(ids);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_ids_agree_with_substring_search() {
        let patterns = [
            "alpha beta",
            "beta gamma",
            "alpha",
            "gamma delta",
            "ab",
            "alpha beta gamma delta",
        ];
        let mut trie = NodeTextTrie::new();
        for (id, pattern) in patterns.iter().enumerate() {
            trie.insert(pattern.as_bytes(), id as u32);
        }
        trie.insert(b"", 99);
        for haystack in [
            "",
            "alpha beta gamma",
            "xx alpha beta gamma delta yy",
            "gamma",
            "abab",
            "alph",
        ] {
            let expected: Vec<u32> = patterns
                .iter()
                .enumerate()
                .filter(|(_, pattern)| haystack.contains(**pattern))
                .map(|(id, _)| id as u32)
                .collect();
            assert_eq!(
                trie.matching_ids(haystack.as_bytes()),
                expected,
                "haystack {haystack:?}"
            );
        }
    }

    #[test]
    fn shared_patterns_report_every_owner() {
        let mut trie = NodeTextTrie::new();
        trie.insert(b"same text", 3);
        trie.insert(b"same text", 7);
        assert_eq!(trie.matching_ids(b"a same text b"), vec![3, 7]);
    }
}
