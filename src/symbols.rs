/// Dorabella Cipher — Symbol Encoding System
///
/// Edward Elgar's 1897 cipher to Dora Penny uses 87 characters drawn from
/// a 24-symbol alphabet: semicircular arcs in 8 orientations × 3 arc counts.
///
/// The 24 symbols form a natural algebraic structure: 8 orientations × 3 arc counts.
/// This 24-element space provides the foundation for the attack engine's
/// frequency analysis and mapping-convergence algorithms.
///
/// Transcription source: Hauer et al. 2021 — the gold-standard machine-readable
/// encoding used for rigorous statistical and musical modeling.

/// The 8 compass directions an arc cluster can face.
/// Labeled A–H in the Hauer transcription convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    A = 0, // Orientation A (Hauer convention)
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
}

impl Direction {
    pub const ALL: [Direction; 8] = [
        Direction::A, Direction::B, Direction::C, Direction::D,
        Direction::E, Direction::F, Direction::G, Direction::H,
    ];

    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Direction::A),
            1 => Some(Direction::B),
            2 => Some(Direction::C),
            3 => Some(Direction::D),
            4 => Some(Direction::E),
            5 => Some(Direction::F),
            6 => Some(Direction::G),
            7 => Some(Direction::H),
            _ => None,
        }
    }

    pub fn label(&self) -> char {
        (b'A' + *self as u8) as char
    }

    /// Degrees clockwise from East (one possible geometric interpretation).
    pub fn degrees(&self) -> f64 {
        (*self as u8) as f64 * 45.0
    }
}

/// A single Dorabella symbol: direction + arc count (1..=3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub direction: Direction,
    pub arcs: u8, // 1, 2, or 3
}

impl Symbol {
    pub fn new(direction: Direction, arcs: u8) -> Self {
        assert!(arcs >= 1 && arcs <= 3, "Arc count must be 1, 2, or 3");
        Symbol { direction, arcs }
    }

    /// Unique index in [0..24), mapping onto the 24-element symbol space.
    /// Encoding: direction_index * 3 + (arc_count - 1)
    pub fn coset_index(&self) -> usize {
        (self.direction as usize) * 3 + (self.arcs as usize - 1)
    }

    /// Reconstruct from coset index.
    pub fn from_coset_index(idx: usize) -> Option<Self> {
        if idx >= 24 { return None; }
        let dir = Direction::from_index((idx / 3) as u8)?;
        let arcs = (idx % 3) as u8 + 1;
        Some(Symbol { direction: dir, arcs })
    }

    /// Hauer-format label, e.g. "F2", "A3".
    pub fn hauer_label(&self) -> String {
        format!("{}{}", self.direction.label(), self.arcs)
    }

    /// Parse from Hauer label like "F2" or "A3".
    pub fn from_hauer(s: &str) -> Option<Self> {
        if s.len() != 2 { return None; }
        let dir_byte = s.as_bytes()[0];
        let arc_byte = s.as_bytes()[1];
        if dir_byte < b'A' || dir_byte > b'H' { return None; }
        let dir = Direction::from_index(dir_byte - b'A')?;
        let arcs = (arc_byte as char).to_digit(10)? as u8;
        if arcs < 1 || arcs > 3 { return None; }
        Some(Symbol { direction: dir, arcs })
    }

    /// Unicode approximation for terminal display.
    pub fn glyph(&self) -> &'static str {
        match (self.direction as u8, self.arcs) {
            (0, 1) => ")",   (0, 2) => "))",  (0, 3) => ")))",
            (1, 1) => "⌝",  (1, 2) => "⌝⌝", (1, 3) => "⌝⌝⌝",
            (2, 1) => "⌢",  (2, 2) => "⌢⌢", (2, 3) => "⌢⌢⌢",
            (3, 1) => "⌜",  (3, 2) => "⌜⌜", (3, 3) => "⌜⌜⌜",
            (4, 1) => "(",   (4, 2) => "((",  (4, 3) => "(((",
            (5, 1) => "⌞",  (5, 2) => "⌞⌞", (5, 3) => "⌞⌞⌞",
            (6, 1) => "⌣",  (6, 2) => "⌣⌣", (6, 3) => "⌣⌣⌣",
            (7, 1) => "⌟",  (7, 2) => "⌟⌟", (7, 3) => "⌟⌟⌟",
            _ => "?",
        }
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hauer_label())
    }
}

// ═══════════════════════════════════════════════════════════════
// THE 87-CHARACTER CIPHERTEXT — Hauer et al. 2021 transcription
// ═══════════════════════════════════════════════════════════════

/// Hauer transcription as raw glyph labels.
pub const HAUER_GLYPHS: &[&str] = &[
    "A2","E3","B2","A3","A1","C2","G1","A3","D1","H2",
    "B3","F2","F1","B1","F2","C3","F2","F2","C2","E3",
    "E3","F2","B1","H1","H2","H1","C1","B3","F3","G1",
    "F2","G1","C2","H1","A3","D1","D2","A3","B2","F2",
    "F2","B2","C2","C1","F1","G1","F2","B3","F2","C2",
    "G2","F3","F1","B1","H1","D1","D1","H1","B3","F3",
    "B2","F3","C2","G2","F3","B2","B1","G2","G3","C1",
    "F3","B2","F2","C2","G2","F1","F3","C1","A3","E3",
    "C1","F3","C2","A3","B1","H1","A3",
];

/// The Dorabella ciphertext as coset indices (0..24).
///
/// Encoded from the Hauer et al. 2021 transcription using:
///   coset_index = (orient_A_through_H) * 3 + (arc_count - 1)
///
/// This gives a perfect injection into the 24-element symbol space.
///
/// Frequency profile (20 of 24 symbols used):
///   F2=11, C2=8, F3=8, A3=7, B2=6, H1=6, B1=5, C1=5,
///   D1=4, B3=4, E3=4, F1=4, G1=4, G2=4, H2=2,
///   A1=1, A2=1, C3=1, D2=1, G3=1
///   Unused: D3(11), E1(12), E2(13), H3(23)
pub const CIPHERTEXT: &[u8] = &[
    // Glyph:  A2 E3 B2 A3 A1 C2 G1 A3 D1 H2  B3 F2 F1 B1 F2 C3 F2 F2 C2 E3
    // Coset:   1 14  4  2  0  7 18  2  9 22   5 16 15  3 16  8 16 16  7 14
                1, 14,  4,  2,  0,  7, 18,  2,  9, 22,  5, 16, 15,  3, 16,  8, 16, 16,  7, 14,
    // Glyph:  E3 F2 B1 H1 H2 H1 C1 B3 F3 G1  F2 G1 C2 H1 A3 D1 D2 A3 B2 F2
    // Coset:  14 16  3 21 22 21  6  5 17 18  16 18  7 21  2  9 10  2  4 16
               14, 16,  3, 21, 22, 21,  6,  5, 17, 18, 16, 18,  7, 21,  2,  9, 10,  2,  4, 16,
    // Glyph:  F2 B2 C2 C1 F1 G1 F2 B3 F2 C2  G2 F3 F1 B1 H1 D1 D1 H1 B3 F3
    // Coset:  16  4  7  6 15 18 16  5 16  7  19 17 15  3 21  9  9 21  5 17
               16,  4,  7,  6, 15, 18, 16,  5, 16,  7, 19, 17, 15,  3, 21,  9,  9, 21,  5, 17,
    // Glyph:  B2 F3 C2 G2 F3 B2 B1 G2 G3 C1  F3 B2 F2 C2 G2 F1 F3 C1 A3 E3
    // Coset:   4 17  7 19 17  4  3 19 20  6  17  4 16  7 19 15 17  6  2 14
                4, 17,  7, 19, 17,  4,  3, 19, 20,  6, 17,  4, 16,  7, 19, 15, 17,  6,  2, 14,
    // Glyph:  C1 F3 C2 A3 B1 H1 A3
    // Coset:   6 17  7  2  3 21  2
                6, 17,  7,  2,  3, 21,  2,
];

/// Number of symbols in the ciphertext.
pub const CIPHER_LEN: usize = 87;

/// Approximate line boundaries from the original note.
/// (Exact boundaries depend on transcription; these follow Hauer.)
pub const LINE_BREAKS: [(usize, usize); 3] = [
    (0, 27),   // Line 1: ~27 symbols
    (27, 53),  // Line 2: ~26 symbols
    (53, 87),  // Line 3: ~34 symbols
];

/// Schmeh proxy letter mapping (for quick substitution-only testing).
/// Maps the 87 coset indices through a simple alphabetic substitution.
/// Top frequency: J=11, F/P=8, C/D/G/N=6.
pub const SCHMEH_PROXY: &str = "ABCDEFGDHAIJKLJMJJFBBJNGOGNIPGJGFQDHRSCJJCFNKGJIJFTPKLQHHQIPCPFUPCLUUNPCJFUKPNDBNPFDLED";

// ═══════════════════════════════════════════════════════════════
// ANALYSIS FUNCTIONS
// ═══════════════════════════════════════════════════════════════

/// How many distinct symbols appear in the ciphertext.
pub fn unique_symbol_count() -> usize {
    let mut seen = [false; 24];
    for &s in CIPHERTEXT {
        seen[s as usize] = true;
    }
    seen.iter().filter(|&&b| b).count()
}

/// Which symbol indices are unused in the ciphertext.
pub fn unused_symbols() -> Vec<usize> {
    let mut seen = [false; 24];
    for &s in CIPHERTEXT {
        seen[s as usize] = true;
    }
    (0..24).filter(|&i| !seen[i]).collect()
}

/// Frequency of each symbol (0..24) in the ciphertext.
pub fn symbol_frequencies() -> [u32; 24] {
    let mut freq = [0u32; 24];
    for &s in CIPHERTEXT {
        freq[s as usize] += 1;
    }
    freq
}

/// Frequency as proportions (sum = 1.0).
pub fn symbol_proportions() -> [f64; 24] {
    let freq = symbol_frequencies();
    let total = CIPHERTEXT.len() as f64;
    let mut props = [0.0f64; 24];
    for i in 0..24 {
        props[i] = freq[i] as f64 / total;
    }
    props
}

/// Sorted frequency table: (coset_index, count), descending.
pub fn sorted_frequencies() -> Vec<(usize, u32)> {
    let freq = symbol_frequencies();
    let mut pairs: Vec<(usize, u32)> = (0..24).map(|i| (i, freq[i])).collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs
}

/// Index of Coincidence for the ciphertext.
/// IC = Σ n_i(n_i-1) / (N(N-1))
/// English monoalphabetic (26 letters) ≈ 0.0667
/// Random 24 symbols ≈ 1/24 ≈ 0.0417
/// Random 26 symbols ≈ 1/26 ≈ 0.0385
pub fn index_of_coincidence() -> f64 {
    let freq = symbol_frequencies();
    let n = CIPHERTEXT.len() as f64;
    let numerator: f64 = freq.iter()
        .map(|&f| f as f64 * (f as f64 - 1.0))
        .sum();
    numerator / (n * (n - 1.0))
}

/// Bigram frequencies (consecutive symbol pairs).
pub fn bigram_frequencies() -> Vec<((u8, u8), u32)> {
    let mut freq = std::collections::HashMap::new();
    for pair in CIPHERTEXT.windows(2) {
        *freq.entry((pair[0], pair[1])).or_insert(0u32) += 1;
    }
    let mut pairs: Vec<_> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs
}

/// Trigram frequencies (consecutive symbol triples).
pub fn trigram_frequencies() -> Vec<((u8, u8, u8), u32)> {
    let mut freq = std::collections::HashMap::new();
    for triple in CIPHERTEXT.windows(3) {
        *freq.entry((triple[0], triple[1], triple[2])).or_insert(0u32) += 1;
    }
    let mut triples: Vec<_> = freq.into_iter().collect();
    triples.sort_by(|a, b| b.1.cmp(&a.1));
    triples
}

/// Convert a mapping (coset_index → letter) and produce the plaintext.
pub fn decrypt_with_mapping(mapping: &[u8; 24]) -> String {
    CIPHERTEXT.iter()
        .map(|&s| mapping[s as usize] as char)
        .collect()
}

/// Build all 24 Symbol objects.
pub fn all_symbols() -> [Symbol; 24] {
    let mut symbols = [Symbol { direction: Direction::A, arcs: 1 }; 24];
    for i in 0..24 {
        symbols[i] = Symbol::from_coset_index(i).unwrap();
    }
    symbols
}

// ═══════════════════════════════════════════════════════════════
// CONTEXTUAL CRIBS — words Elgar might have written to Dora
// ═══════════════════════════════════════════════════════════════

/// Likely plaintext fragments based on the Elgar–Penny relationship.
pub const CONTEXTUAL_CRIBS: &[&str] = &[
    // Personal address
    "DORA", "DORABELLA", "PENNY", "DEAR", "MISS",
    "DEARDORA", "MYDEAR", "DEARMISS",
    // Musical terms Elgar would use
    "ENIGMA", "THEME", "MUSIC", "VARIATION", "ADAGIO",
    "ALLEGRO", "ANDANTE", "PIANO", "FORTE", "TEMPO",
    // Common letter closings (Victorian era)
    "YOURS", "TRULY", "EVER", "SINCERELY", "AFFECTIONATELY",
    "EDWARD", "ELGAR", "EE",
    // Contextual words from their social circle
    "FESTIVAL", "CONCERT", "MALVERN", "WOLVERHAMPTON",
    "WEDNESDAY", "THURSDAY", "FRIDAY", "SATURDAY",
    "TOMORROW", "TONIGHT", "YESTERDAY",
    // Common short words
    "THE", "AND", "BUT", "NOT", "YOU", "YOUR", "WITH",
    "FOR", "FROM", "HAVE", "THIS", "THAT", "WILL",
    "COME", "MEET", "HOPE", "LOVE", "LIKE", "KNOW",
];

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ciphertext_length() {
        assert_eq!(CIPHERTEXT.len(), CIPHER_LEN);
        assert_eq!(CIPHERTEXT.len(), 87);
    }

    #[test]
    fn test_all_symbols_in_range() {
        for &s in CIPHERTEXT {
            assert!(s < 24, "Symbol {} out of range", s);
        }
    }

    #[test]
    fn test_hauer_glyphs_match_cosets() {
        assert_eq!(HAUER_GLYPHS.len(), CIPHER_LEN);
        for (i, glyph) in HAUER_GLYPHS.iter().enumerate() {
            let sym = Symbol::from_hauer(glyph).unwrap();
            assert_eq!(
                sym.coset_index(), CIPHERTEXT[i] as usize,
                "Mismatch at position {}: {} → coset {} but expected {}",
                i, glyph, sym.coset_index(), CIPHERTEXT[i]
            );
        }
    }

    #[test]
    fn test_coset_index_roundtrip() {
        for i in 0..24 {
            let sym = Symbol::from_coset_index(i).unwrap();
            assert_eq!(sym.coset_index(), i);
        }
    }

    #[test]
    fn test_hauer_parse_roundtrip() {
        for i in 0..24 {
            let sym = Symbol::from_coset_index(i).unwrap();
            let label = sym.hauer_label();
            let parsed = Symbol::from_hauer(&label).unwrap();
            assert_eq!(sym, parsed);
        }
    }

    #[test]
    fn test_unique_symbol_count() {
        let count = unique_symbol_count();
        assert_eq!(count, 20, "Hauer transcription uses exactly 20 of 24 symbols");
    }

    #[test]
    fn test_unused_symbols() {
        let unused = unused_symbols();
        // D3=11, E1=12, E2=13, H3=23
        assert_eq!(unused, vec![11, 12, 13, 23]);
    }

    #[test]
    fn test_symbol_frequencies_sum() {
        let freq = symbol_frequencies();
        let total: u32 = freq.iter().sum();
        assert_eq!(total, CIPHER_LEN as u32);
    }

    #[test]
    fn test_most_frequent_is_f2() {
        let sorted = sorted_frequencies();
        // F2 (coset 16) should be most frequent at 11 occurrences
        assert_eq!(sorted[0].0, 16); // coset index for F2
        assert_eq!(sorted[0].1, 11);
    }

    #[test]
    fn test_proportions_sum() {
        let props = symbol_proportions();
        let total: f64 = props.iter().sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ic_reasonable() {
        let ic = index_of_coincidence();
        assert!(ic > 0.03, "IC {} too low", ic);
        assert!(ic < 0.15, "IC {} too high", ic);
    }

    #[test]
    fn test_decrypt_with_identity() {
        let mut mapping = [0u8; 24];
        for i in 0..24 {
            mapping[i] = b'A' + (i as u8 % 26);
        }
        let plaintext = decrypt_with_mapping(&mapping);
        assert_eq!(plaintext.len(), 87);
    }

    #[test]
    fn test_schmeh_proxy_length() {
        assert_eq!(SCHMEH_PROXY.len(), CIPHER_LEN);
    }
}
