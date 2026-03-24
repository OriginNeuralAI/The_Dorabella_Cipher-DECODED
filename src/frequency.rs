/// Dorabella Cipher — Frequency Analysis & Language Scoring
///
/// Scores candidate decryptions against English letter statistics.
/// Combines chi-squared unigram, log-probability bigram/trigram,
/// word-boundary heuristics, and dictionary hit scoring.

// ═══════════════════════════════════════════════════════════════
// ENGLISH LETTER FREQUENCIES (from large corpora)
// ═══════════════════════════════════════════════════════════════

/// English single-letter frequencies (A=0, B=1, ..., Z=25).
pub const ENGLISH_FREQ: [f64; 26] = [
    0.08167, // A
    0.01492, // B
    0.02782, // C
    0.04253, // D
    0.12702, // E
    0.02228, // F
    0.02015, // G
    0.06094, // H
    0.06966, // I
    0.00153, // J
    0.00772, // K
    0.04025, // L
    0.02406, // M
    0.06749, // N
    0.07507, // O
    0.01929, // P
    0.00095, // Q
    0.05987, // R
    0.06327, // S
    0.09056, // T
    0.02758, // U
    0.00978, // V
    0.02360, // W
    0.00150, // X
    0.01974, // Y
    0.00074, // Z
];

/// Sorted letter frequency order (most to least frequent).
pub const FREQ_ORDER: &[u8] = b"ETAOINSRHLDCUMWFGYPBVKJXQZ";

/// English bigram log-probabilities (top 50 pairs).
/// Stored as (byte pair, log10 probability).
static BIGRAM_LOG_PROBS: &[([u8; 2], f64)] = &[
    (*b"TH", -1.30), (*b"HE", -1.35), (*b"IN", -1.52), (*b"ER", -1.55),
    (*b"AN", -1.60), (*b"RE", -1.67), (*b"ON", -1.74), (*b"AT", -1.77),
    (*b"EN", -1.79), (*b"ND", -1.82), (*b"TI", -1.85), (*b"ES", -1.86),
    (*b"OR", -1.88), (*b"TE", -1.90), (*b"OF", -1.91), (*b"ED", -1.93),
    (*b"IS", -1.94), (*b"IT", -1.95), (*b"AL", -1.96), (*b"AR", -1.97),
    (*b"ST", -1.98), (*b"TO", -2.00), (*b"NT", -2.02), (*b"NG", -2.05),
    (*b"SE", -2.07), (*b"HA", -2.08), (*b"AS", -2.10), (*b"OU", -2.12),
    (*b"IO", -2.13), (*b"LE", -2.15), (*b"VE", -2.17), (*b"CO", -2.18),
    (*b"ME", -2.20), (*b"DE", -2.22), (*b"HI", -2.24), (*b"RI", -2.25),
    (*b"RO", -2.27), (*b"IC", -2.28), (*b"NE", -2.30), (*b"EA", -2.31),
    (*b"RA", -2.33), (*b"CE", -2.35), (*b"LI", -2.37), (*b"CH", -2.38),
    (*b"LL", -2.40), (*b"BE", -2.42), (*b"MA", -2.44), (*b"SI", -2.45),
    (*b"OM", -2.47), (*b"UR", -2.48),
];

/// English trigram log-probabilities (top 30 triples).
static TRIGRAM_LOG_PROBS: &[([u8; 3], f64)] = &[
    (*b"THE", -2.10), (*b"AND", -2.50), (*b"ING", -2.65), (*b"HER", -2.85),
    (*b"HAT", -2.90), (*b"HIS", -2.95), (*b"THA", -3.00), (*b"ERE", -3.05),
    (*b"FOR", -3.08), (*b"ENT", -3.10), (*b"ION", -3.12), (*b"TER", -3.15),
    (*b"WAS", -3.18), (*b"YOU", -3.20), (*b"ITH", -3.22), (*b"VER", -3.25),
    (*b"ALL", -3.27), (*b"WIT", -3.30), (*b"THI", -3.32), (*b"TIO", -3.35),
    (*b"OUR", -3.38), (*b"OUS", -3.40), (*b"NOT", -3.42), (*b"ARE", -3.45),
    (*b"BUT", -3.47), (*b"HAD", -3.50), (*b"ONE", -3.52), (*b"OUR", -3.54),
    (*b"OUT", -3.56), (*b"DAY", -3.58),
];

// ═══════════════════════════════════════════════════════════════
// SCORING FUNCTIONS
// ═══════════════════════════════════════════════════════════════

/// Chi-squared statistic measuring deviation from English letter frequencies.
/// Lower is better (0 = perfect match to English).
pub fn chi_squared(text: &str) -> f64 {
    let mut counts = [0u32; 26];
    let mut total = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            counts[(ch.to_ascii_uppercase() as u8 - b'A') as usize] += 1;
            total += 1;
        }
    }
    if total == 0 { return f64::MAX; }

    let n = total as f64;
    let mut chi2 = 0.0;
    for i in 0..26 {
        let observed = counts[i] as f64;
        let expected = ENGLISH_FREQ[i] * n;
        if expected > 0.0 {
            chi2 += (observed - expected).powi(2) / expected;
        }
    }
    chi2
}

/// Bigram log-probability score. Higher (less negative) is more English-like.
pub fn bigram_score(text: &str) -> f64 {
    let bytes: Vec<u8> = text.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if bytes.len() < 2 { return -1000.0; }

    let default_log = -4.0; // Rare bigram penalty
    let mut score = 0.0;
    for pair in bytes.windows(2) {
        let key = [pair[0], pair[1]];
        let lp = BIGRAM_LOG_PROBS.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(default_log);
        score += lp;
    }
    score / (bytes.len() - 1) as f64 // Normalize by length
}

/// Trigram log-probability score. Higher is more English-like.
pub fn trigram_score(text: &str) -> f64 {
    let bytes: Vec<u8> = text.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if bytes.len() < 3 { return -1000.0; }

    let default_log = -5.0;
    let mut score = 0.0;
    for triple in bytes.windows(3) {
        let key = [triple[0], triple[1], triple[2]];
        let lp = TRIGRAM_LOG_PROBS.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(default_log);
        score += lp;
    }
    score / (bytes.len() - 2) as f64
}

/// Combined ensemble score (higher = more likely English).
/// Weights: chi-squared (inverted), bigram, trigram, word hits.
pub fn ensemble_score(text: &str) -> f64 {
    let chi2 = chi_squared(text);
    let bi = bigram_score(text);
    let tri = trigram_score(text);
    let words = word_hit_score(text);

    // Normalize chi-squared: perfect English ≈ 20-40 for 87 chars
    // Invert so lower chi2 → higher score
    let chi2_component = -chi2 / 50.0;

    // Weights tuned for 87-char text
    let score = chi2_component * 2.0  // Frequency match (most reliable)
        + bi * 3.0                    // Bigram patterns
        + tri * 2.0                   // Trigram patterns
        + words * 5.0;               // Dictionary hits (strong signal)

    score
}

/// Count how many common English words appear as substrings.
/// Returns a normalized score (0.0 = none, 1.0 = dense hits).
pub fn word_hit_score(text: &str) -> f64 {
    let upper = text.to_ascii_uppercase();

    // Common English words to search for (2-8 letters)
    const WORDS: &[&str] = &[
        "THE", "AND", "FOR", "ARE", "BUT", "NOT", "YOU", "ALL",
        "HER", "WAS", "ONE", "OUR", "OUT", "HAS", "HIS", "HOW",
        "ITS", "MAY", "NEW", "NOW", "OLD", "SEE", "WAY", "WHO",
        "DID", "GET", "HAS", "HIM", "LET", "SAY", "SHE", "TOO",
        "HAVE", "THAT", "WITH", "THIS", "WILL", "YOUR", "FROM",
        "THEY", "BEEN", "COME", "DEAR", "DORA", "EVER", "HOPE",
        "KNOW", "LIKE", "LOVE", "MEET", "MISS", "MUCH", "MUST",
        "VERY", "WHEN", "HERE", "THERE", "WHICH", "WOULD", "COULD",
        "MUSIC", "THEME", "PENNY",
    ];

    let mut hits = 0u32;
    for word in WORDS {
        if upper.contains(word) {
            hits += word.len() as u32; // Weight by word length
        }
    }

    // Normalize: max plausible coverage ≈ 60 chars of 87
    (hits as f64 / 60.0).min(1.0)
}

/// Compute the Index of Coincidence for a plaintext string.
/// English ≈ 0.0667, random ≈ 0.0385.
pub fn text_ic(text: &str) -> f64 {
    let mut counts = [0u32; 26];
    let mut total = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            counts[(ch.to_ascii_uppercase() as u8 - b'A') as usize] += 1;
            total += 1;
        }
    }
    if total <= 1 { return 0.0; }
    let n = total as f64;
    let numerator: f64 = counts.iter()
        .map(|&f| f as f64 * (f as f64 - 1.0))
        .sum();
    numerator / (n * (n - 1.0))
}

/// Frequency rank correlation: how well does the observed frequency order
/// match English frequency order? Returns Spearman's ρ (−1 to +1).
pub fn frequency_rank_correlation(text: &str) -> f64 {
    let mut counts = [0u32; 26];
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            counts[(ch.to_ascii_uppercase() as u8 - b'A') as usize] += 1;
        }
    }

    // Rank the observed frequencies
    let mut observed_order: Vec<usize> = (0..26).collect();
    observed_order.sort_by(|&a, &b| counts[b].cmp(&counts[a]));

    // Rank the expected frequencies
    let mut expected_order: Vec<usize> = (0..26).collect();
    expected_order.sort_by(|&a, &b| ENGLISH_FREQ[b].partial_cmp(&ENGLISH_FREQ[a]).unwrap());

    // Compute rank arrays
    let mut obs_rank = [0usize; 26];
    let mut exp_rank = [0usize; 26];
    for (rank, &letter) in observed_order.iter().enumerate() {
        obs_rank[letter] = rank;
    }
    for (rank, &letter) in expected_order.iter().enumerate() {
        exp_rank[letter] = rank;
    }

    // Spearman's ρ = 1 - 6Σd² / (n(n²-1))
    let mut sum_d2 = 0.0;
    for i in 0..26 {
        let d = obs_rank[i] as f64 - exp_rank[i] as f64;
        sum_d2 += d * d;
    }
    let n = 26.0;
    1.0 - (6.0 * sum_d2) / (n * (n * n - 1.0))
}

/// Check if a candidate plaintext contains "impossible" English patterns.
/// Returns a penalty (0.0 = fine, negative = suspicious).
pub fn impossible_pattern_penalty(text: &str) -> f64 {
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut penalty = 0.0;

    // Check for runs of 4+ consonants (rare in English)
    const VOWELS: &[u8] = b"AEIOU";
    let mut consonant_run = 0;
    for &b in bytes {
        if b.is_ascii_alphabetic() && !VOWELS.contains(&b) {
            consonant_run += 1;
            if consonant_run >= 4 {
                penalty -= 1.0;
            }
        } else {
            consonant_run = 0;
        }
    }

    // Check for repeated letters (3+ in a row — very rare)
    for window in bytes.windows(3) {
        if window[0] == window[1] && window[1] == window[2] {
            penalty -= 2.0;
        }
    }

    // Check vowel ratio (English ≈ 38-42% vowels)
    let alpha_count = bytes.iter().filter(|b| b.is_ascii_alphabetic()).count();
    let vowel_count = bytes.iter().filter(|b| VOWELS.contains(b)).count();
    if alpha_count > 0 {
        let ratio = vowel_count as f64 / alpha_count as f64;
        if ratio < 0.25 || ratio > 0.55 {
            penalty -= 3.0;
        }
    }

    penalty
}

// ═══════════════════════════════════════════════════════════════
// ELGAR-SPEAK SCORING
// ═══════════════════════════════════════════════════════════════

/// Score text for "Elgar speak" — the private language Edward Elgar used
/// in informal correspondence: backslang, abbreviations, portmanteaus,
/// musical terms, and Victorian slang.
///
/// Source: Dora Penny's memoir, Kevin Jones's research, Wase 2023.
pub fn elgar_speak_score(text: &str) -> f64 {
    let upper = text.to_ascii_uppercase();
    let mut score = 0.0;

    // ── Backslang detection ─────────────────────────────────
    // Elgar frequently reversed words: "dog" → "god", "live" → "evil"
    score += backslang_score(&upper);

    // ── Abbreviations & nicknames ───────────────────────────
    // Victorian informal: dropping vowels, truncating, pet names
    score += abbreviation_score(&upper);

    // ── Musical vocabulary ──────────────────────────────────
    // Elgar would naturally use musical terms with Dora
    score += musical_vocabulary_score(&upper);

    // ── Victorian letter-writing patterns ────────────────────
    score += victorian_letter_score(&upper);

    // ── Relaxed English: allow non-standard but pronounceable ─
    score += pronounceability_score(&upper);

    score
}

/// Detect reversed common English words (Elgar's backslang habit).
fn backslang_score(text: &str) -> f64 {
    const BACKSLANG_PAIRS: &[(&str, &str)] = &[
        ("EVOL", "LOVE"), ("RAED", "DEAR"), ("AROD", "DORA"),
        ("CISUM", "MUSIC"), ("YREVE", "EVERY"), ("DOOG", "GOOD"),
        ("THGIN", "NIGHT"), ("GNINROM", "MORNING"), ("EMOH", "HOME"),
        ("EMOC", "COME"), ("EPOH", "HOPE"), ("WONK", "KNOW"),
        ("TEEM", "MEET"), ("NEVE", "EVEN"), ("TSRIF", "FIRST"),
        ("TSUJ", "JUST"), ("YLNO", "ONLY"), ("HCUM", "MUCH"),
        ("RETTEB", "BETTER"), ("RETFA", "AFTER"),
        // Elgar's known reversals
        ("NAGILE", "ELIGAN"), ("RAGLE", "ELGAR"),
    ];

    let mut hits = 0.0;
    for (backslang, _forward) in BACKSLANG_PAIRS {
        if text.contains(backslang) {
            hits += backslang.len() as f64 * 0.3;
        }
    }
    hits
}

/// Score for Victorian-era abbreviations and informal patterns.
fn abbreviation_score(text: &str) -> f64 {
    // Common abbreviations in Victorian informal letters
    const ABBREVS: &[&str] = &[
        "YR", "YRS", "WD", "CD", "SHD", "THO", "THNK",
        "BT", "NT", "WH", "TH", "HV", "WL", "SH",
        "MSG", "LTR", "MTG", "EVG", "MRN", "AFT",
        "SPL", "WDR", "GRT", "LVL", "BEA", "FUL",
        // Elgar's known nicknames and abbreviations
        "EE", "DORAB", "DRAB", "WEP", "BRAMO",
    ];

    let mut score = 0.0;
    for abbrev in ABBREVS {
        if text.contains(abbrev) {
            score += abbrev.len() as f64 * 0.15;
        }
    }
    score
}

/// Score for musical vocabulary.
fn musical_vocabulary_score(text: &str) -> f64 {
    const MUSICAL: &[&str] = &[
        // Italian musical terms Elgar used daily
        "ADAGIO", "ALLEGRO", "ANDANTE", "FORTE", "PIANO",
        "DOLCE", "VIVACE", "PRESTO", "LARGO", "TEMPO",
        "SCHERZO", "FUGUE", "SONATA", "ARIA", "CODA",
        "STACCATO", "LEGATO", "CANTAB", "MEZZA", "SOTTO",
        // Musical terms in English
        "THEME", "VARIATION", "ENIGMA", "MELODY", "CHORD",
        "SCORE", "NOTE", "FLAT", "SHARP", "KEY",
        "MINOR", "MAJOR", "SCALE", "SUITE", "MARCH",
        "CONCERT", "RECITAL", "FESTIVAL", "ORCHESTRA",
        // Note names
        "CDEF", "EFGA", "GABC", "DEFC",
    ];

    let mut score = 0.0;
    for word in MUSICAL {
        if text.contains(word) {
            score += word.len() as f64 * 0.25;
        }
    }
    score
}

/// Score for Victorian letter-writing patterns.
fn victorian_letter_score(text: &str) -> f64 {
    const VICTORIAN: &[&str] = &[
        // Greetings and closings
        "DEAR", "DEAREST", "DARLING", "BELOVED",
        "YOURS", "TRULY", "EVER", "FAITHFULLY", "AFFECTION",
        "FONDLY", "WARMLY", "KINDLY",
        // Common letter phrases
        "IWRITE", "IHOPE", "ITRUST", "IPRAY",
        "DELIGHTED", "CHARMED", "PLEASED", "SORRY",
        "LOOKING", "FORWARD", "SEEING", "MEETING",
        // Temporal references
        "TOMORROW", "YESTERDAY", "TODAY", "TONIGHT",
        "WEDNESDAY", "THURSDAY", "FRIDAY", "SATURDAY",
        "SUNDAY", "MONDAY", "TUESDAY",
        // Places Elgar and Penny frequented
        "MALVERN", "WOLVERHAMPTON", "LONDON", "BIRCHWOOD",
        "HASFIELD", "NORBURY",
    ];

    let mut score = 0.0;
    for word in VICTORIAN {
        if text.contains(word) {
            score += word.len() as f64 * 0.2;
        }
    }
    score
}

/// Score for pronounceability — Elgar-speak is readable aloud even if
/// not standard English. Penalize truly unpronounceable sequences less
/// harshly than the standard English scorer.
fn pronounceability_score(text: &str) -> f64 {
    let bytes = text.as_bytes();
    let mut score = 0.0;

    // Reward consonant-vowel alternation (pronounceable)
    const VOWELS: &[u8] = b"AEIOU";
    let mut cv_alternations = 0u32;
    for pair in bytes.windows(2) {
        let a_vowel = VOWELS.contains(&pair[0]);
        let b_vowel = VOWELS.contains(&pair[1]);
        if a_vowel != b_vowel {
            cv_alternations += 1;
        }
    }
    if bytes.len() > 1 {
        let ratio = cv_alternations as f64 / (bytes.len() - 1) as f64;
        // Good CV alternation: 0.5-0.7
        if ratio > 0.4 && ratio < 0.8 {
            score += 2.0;
        }
    }

    // Reward common consonant clusters (pronounceable but non-standard)
    const CLUSTERS: &[&[u8]] = &[
        b"TH", b"SH", b"CH", b"WH", b"PH", b"GH", b"CK",
        b"ST", b"SP", b"SK", b"SL", b"SM", b"SN", b"SW",
        b"BL", b"BR", b"CL", b"CR", b"DR", b"FL", b"FR",
        b"GL", b"GR", b"PL", b"PR", b"TR", b"WR", b"SC",
        b"NG", b"NK", b"NT", b"ND", b"LT", b"LD", b"RN",
    ];
    for cluster in CLUSTERS {
        let cs: &[u8] = cluster;
        for w in bytes.windows(cs.len()) {
            if w == cs {
                score += 0.3;
            }
        }
    }

    score
}

/// Combined "Elgar" ensemble score: standard English + Elgar-speak bonus.
/// Use this instead of ensemble_score() for Phase 10.
pub fn elgar_ensemble_score(text: &str) -> f64 {
    let standard = ensemble_score(text);
    let elgar = elgar_speak_score(text);
    standard + elgar * 2.0  // Weight Elgar-speak as significant bonus
}

// ═══════════════════════════════════════════════════════════════
// FREQUENCY-BASED INITIAL MAPPING
// ═══════════════════════════════════════════════════════════════

/// Generate an initial mapping by matching symbol frequencies to English
/// letter frequencies (most frequent symbol → most frequent letter, etc.).
///
/// Returns a mapping: coset_index → ASCII letter byte.
pub fn frequency_matched_mapping(cipher_freqs: &[u32; 24]) -> [u8; 24] {
    // Sort symbols by frequency (descending)
    let mut sym_order: Vec<usize> = (0..24).collect();
    sym_order.sort_by(|&a, &b| cipher_freqs[b].cmp(&cipher_freqs[a]));

    // Map most-frequent symbol to most-frequent English letter
    let mut mapping = [b'?'; 24];
    for (rank, &sym_idx) in sym_order.iter().enumerate() {
        if rank < 26 {
            mapping[sym_idx] = FREQ_ORDER[rank];
        } else {
            // Overflow: map to least-common letters
            mapping[sym_idx] = b'Z';
        }
    }
    mapping
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_freq_sums_to_one() {
        let sum: f64 = ENGLISH_FREQ.iter().sum();
        assert!((sum - 1.0).abs() < 0.01, "English freq sum = {}", sum);
    }

    #[test]
    fn test_chi_squared_english_text() {
        let english = "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG";
        let chi2 = chi_squared(english);
        // Pangrams have intentionally odd letter distribution (every letter once)
        // so chi2 can be 100-150; real prose is lower. Threshold 200 is generous.
        assert!(chi2 < 200.0, "Pangram chi2 = {} (should be under 200)", chi2);
    }

    #[test]
    fn test_chi_squared_random_text() {
        // Heavily skewed text
        let skewed = "AAAAAAAAAAAABBBBBBBBBBBB";
        let chi2 = chi_squared(skewed);
        assert!(chi2 > 100.0, "Skewed chi2 = {} (should be high)", chi2);
    }

    #[test]
    fn test_bigram_score_english() {
        let english = "THEQUICKBROWNFOX";
        let random = "XZQJKVBWMFP";
        let english_score = bigram_score(english);
        let random_score = bigram_score(random);
        assert!(english_score > random_score,
            "English bigram {} should beat random {}",
            english_score, random_score);
    }

    #[test]
    fn test_word_hit_score() {
        let text = "THEDEARMUSIC";
        let score = word_hit_score(text);
        assert!(score > 0.0, "Should find THE, DEAR, MUSIC");
    }

    #[test]
    fn test_text_ic_english() {
        let english = "THEREWASAMANFROMYORKWHOLOOKEDLIKEASKINNYORKSHIREPORKHEATEPI\
                       ESANDCAKESTHREETIMESONTHEWEEKENDANDHECOULDONLYWALKWITHA";
        let ic = text_ic(english);
        assert!(ic > 0.05, "English text IC = {} (should be > 0.05)", ic);
    }

    #[test]
    fn test_frequency_matched_mapping() {
        let freqs = super::super::symbols::symbol_frequencies();
        let mapping = frequency_matched_mapping(&freqs);
        // Most frequent symbol (F2, coset 16, count=11) should map to 'E'
        assert_eq!(mapping[16], b'E', "Most frequent should map to E");
    }

    #[test]
    fn test_ensemble_score_english_beats_random() {
        let english = "THEMANWENTTOTHEMARKET";
        let random = "ZXQJVBFKMWPLRGDY";
        assert!(ensemble_score(english) > ensemble_score(random));
    }

    #[test]
    fn test_elgar_speak_detects_backslang() {
        let text = "EVOLDEARFRIEND";  // Contains EVOL (reversed LOVE) and DEAR
        let score = elgar_speak_score(text);
        assert!(score > 0.0, "Should detect backslang EVOL and word DEAR, got {}", score);
    }

    #[test]
    fn test_elgar_speak_detects_musical() {
        let text = "THEADAGIOISLOVELY";
        let score = elgar_speak_score(text);
        assert!(score > 0.0, "Should detect ADAGIO, got {}", score);
    }

    #[test]
    fn test_elgar_ensemble_beats_standard_for_mixed() {
        // Text with musical terms should score higher on Elgar ensemble
        let musical = "THEENIGMATHEMEISLOVELY";
        let elgar = elgar_ensemble_score(musical);
        let standard = ensemble_score(musical);
        assert!(elgar >= standard,
            "Elgar ensemble {} should be >= standard {} for musical text",
            elgar, standard);
    }
}
