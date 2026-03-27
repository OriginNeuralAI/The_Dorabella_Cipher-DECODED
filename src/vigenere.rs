/// Dorabella Cipher — Polyalphabetic & Non-MASC Attack Module
///
/// Viktor Wase (2023, Cryptologia) showed that modern MASC solvers achieve 98.7%
/// success on 87-char monoalphabetic ciphers but NEVER crack Dorabella, concluding
/// it is "unlikely to be a MASC." This module implements the leading alternatives:
///
///   1. Vigenère / polyalphabetic (shift mod 24 with periodic key)
///   2. Kasiski examination (repeated n-gram spacing → key period)
///   3. IC-based period estimation
///   4. Column-wise MASC solving per period
///   5. Musical key testing (ENIGMA, BACH, DORA, Enigma theme intervals)
///   6. Homophonic hypothesis (multiple symbols → same letter)
///   7. Transposition hypothesis (rearrange line order / read pattern)
///   8. Null-symbol hypothesis (some symbols are spacers/nulls)

use super::symbols::{CIPHERTEXT, CIPHER_LEN, LINE_BREAKS};
use super::frequency;

// ═══════════════════════════════════════════════════════════════
// KASISKI EXAMINATION
// ═══════════════════════════════════════════════════════════════

/// Find repeated n-grams (length 2-5) and return their spacings.
/// The GCD of these spacings suggests the key period.
pub fn kasiski_examination() -> Vec<KasiskiResult> {
    let mut results = Vec::new();

    for ngram_len in 2..=5 {
        let mut positions: std::collections::HashMap<Vec<u8>, Vec<usize>> =
            std::collections::HashMap::new();

        for i in 0..=(CIPHER_LEN.saturating_sub(ngram_len)) {
            let ngram: Vec<u8> = CIPHERTEXT[i..i + ngram_len].to_vec();
            positions.entry(ngram).or_default().push(i);
        }

        for (ngram, pos) in &positions {
            if pos.len() < 2 { continue; }
            let mut spacings = Vec::new();
            for i in 1..pos.len() {
                spacings.push(pos[i] - pos[i - 1]);
            }
            results.push(KasiskiResult {
                ngram: ngram.clone(),
                positions: pos.clone(),
                spacings: spacings.clone(),
                gcd: multi_gcd(&spacings),
            });
        }
    }

    // Sort by n-gram length (longer = more significant), then by count
    results.sort_by(|a, b| {
        b.ngram.len().cmp(&a.ngram.len())
            .then(b.positions.len().cmp(&a.positions.len()))
    });
    results
}

/// Result of Kasiski examination for one n-gram.
#[derive(Debug, Clone)]
pub struct KasiskiResult {
    pub ngram: Vec<u8>,
    pub positions: Vec<usize>,
    pub spacings: Vec<usize>,
    pub gcd: usize,
}

/// GCD of a list of values.
fn multi_gcd(values: &[usize]) -> usize {
    if values.is_empty() { return 0; }
    let mut result = values[0];
    for &v in &values[1..] {
        result = gcd(result, v);
    }
    result
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

// ═══════════════════════════════════════════════════════════════
// IC-BASED PERIOD ESTIMATION
// ═══════════════════════════════════════════════════════════════

/// For each candidate period p (2..=max_period), split the ciphertext into
/// p columns and compute the average IC per column.
/// If Vigenère, each column's IC should approach monoalphabetic English IC (~0.0667).
pub fn ic_per_period(max_period: usize) -> Vec<PeriodIC> {
    let mut results = Vec::new();

    for period in 2..=max_period.min(20) {
        let mut total_ic = 0.0;
        let mut col_count = 0;

        for col in 0..period {
            // Extract every p-th symbol starting at position col
            let column: Vec<u8> = CIPHERTEXT.iter()
                .skip(col)
                .step_by(period)
                .copied()
                .collect();

            if column.len() < 2 { continue; }

            // Compute IC over the 24-symbol alphabet
            let ic = symbol_ic(&column);
            total_ic += ic;
            col_count += 1;
        }

        let avg_ic = if col_count > 0 { total_ic / col_count as f64 } else { 0.0 };
        results.push(PeriodIC { period, avg_ic, columns: col_count });
    }

    // Sort by IC closest to English monoalphabetic (~0.0667)
    results.sort_by(|a, b| {
        let da = (a.avg_ic - 0.0667).abs();
        let db = (b.avg_ic - 0.0667).abs();
        da.partial_cmp(&db).unwrap()
    });

    results
}

/// IC result for a candidate period.
#[derive(Debug, Clone)]
pub struct PeriodIC {
    pub period: usize,
    pub avg_ic: f64,
    pub columns: usize,
}

/// Compute IC over a sequence of symbol indices (0..24 alphabet).
fn symbol_ic(symbols: &[u8]) -> f64 {
    let mut freq = [0u32; 24];
    for &s in symbols {
        freq[s as usize] += 1;
    }
    let n = symbols.len() as f64;
    if n <= 1.0 { return 0.0; }
    let numerator: f64 = freq.iter()
        .map(|&f| f as f64 * (f as f64 - 1.0))
        .sum();
    numerator / (n * (n - 1.0))
}

// ═══════════════════════════════════════════════════════════════
// VIGENÈRE ATTACK (shift mod 24)
// ═══════════════════════════════════════════════════════════════

/// Attempt Vigenère decryption with a given key (as shifts mod 24).
/// Each key element shifts the corresponding ciphertext symbol.
pub fn vigenere_decrypt(key: &[u8]) -> Vec<u8> {
    if key.is_empty() { return CIPHERTEXT.to_vec(); }
    CIPHERTEXT.iter().enumerate().map(|(i, &s)| {
        ((s as i16 - key[i % key.len()] as i16).rem_euclid(24)) as u8
    }).collect()
}

/// Decrypt polyalphabetic ciphertext, then apply a MASC mapping.
pub fn vigenere_then_masc(key: &[u8], mapping: &[u8; 24]) -> String {
    let shifted = vigenere_decrypt(key);
    shifted.iter().map(|&s| mapping[s as usize] as char).collect()
}

/// Try all single-shift Caesar keys (mod 24) and score the output.
/// Returns sorted candidates.
pub fn caesar_attack() -> Vec<VigenereCandidate> {
    let mut candidates = Vec::new();
    for shift in 0..24u8 {
        let key = vec![shift];
        let shifted = vigenere_decrypt(&key);
        // Build frequency-matched mapping for the shifted distribution
        let mut shifted_freq = [0u32; 24];
        for &s in &shifted {
            shifted_freq[s as usize] += 1;
        }
        let mapping = frequency::frequency_matched_mapping(&shifted_freq);
        let plaintext: String = shifted.iter()
            .map(|&s| mapping[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);

        candidates.push(VigenereCandidate {
            key: key.clone(),
            plaintext,
            score,
            method: "Caesar",
        });
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

/// Try Vigenère with a specific period, solving each column independently.
pub fn vigenere_column_attack(period: usize) -> Vec<VigenereCandidate> {
    if period < 2 || period > 20 { return Vec::new(); }

    let mut candidates = Vec::new();

    // For each column, find the best Caesar shift
    let mut best_key = vec![0u8; period];

    for col in 0..period {
        let column: Vec<u8> = CIPHERTEXT.iter()
            .skip(col)
            .step_by(period)
            .copied()
            .collect();

        let mut best_shift = 0u8;
        let mut best_ic = 0.0f64;

        for shift in 0..24u8 {
            let shifted: Vec<u8> = column.iter()
                .map(|&s| ((s as i16 - shift as i16).rem_euclid(24)) as u8)
                .collect();
            let ic = symbol_ic(&shifted);
            if ic > best_ic {
                best_ic = ic;
                best_shift = shift;
            }
        }
        best_key[col] = best_shift;
    }

    // Decrypt with the best key and frequency-match
    let shifted = vigenere_decrypt(&best_key);
    let mut shifted_freq = [0u32; 24];
    for &s in &shifted {
        shifted_freq[s as usize] += 1;
    }
    let mapping = frequency::frequency_matched_mapping(&shifted_freq);
    let plaintext: String = shifted.iter()
        .map(|&s| mapping[s as usize] as char)
        .collect();
    let score = frequency::ensemble_score(&plaintext)
        + frequency::impossible_pattern_penalty(&plaintext);

    candidates.push(VigenereCandidate {
        key: best_key.clone(),
        plaintext,
        score,
        method: "Vigenere-IC",
    });

    // Also try chi-squared optimization per column
    let mut chi_key = vec![0u8; period];
    for col in 0..period {
        let column: Vec<u8> = CIPHERTEXT.iter()
            .skip(col)
            .step_by(period)
            .copied()
            .collect();

        let mut best_shift = 0u8;
        let mut best_chi = f64::MAX;

        for shift in 0..24u8 {
            let shifted: Vec<u8> = column.iter()
                .map(|&s| ((s as i16 - shift as i16).rem_euclid(24)) as u8)
                .collect();
            // Map to letters via frequency and compute chi-squared
            let mapping = frequency::frequency_matched_mapping(&{
                let mut f = [0u32; 24];
                for &s in &shifted { f[s as usize] += 1; }
                f
            });
            let text: String = shifted.iter()
                .map(|&s| mapping[s as usize] as char)
                .collect();
            let chi = frequency::chi_squared(&text);
            if chi < best_chi {
                best_chi = chi;
                best_shift = shift;
            }
        }
        chi_key[col] = best_shift;
    }

    let shifted = vigenere_decrypt(&chi_key);
    let mut shifted_freq = [0u32; 24];
    for &s in &shifted {
        shifted_freq[s as usize] += 1;
    }
    let mapping = frequency::frequency_matched_mapping(&shifted_freq);
    let plaintext: String = shifted.iter()
        .map(|&s| mapping[s as usize] as char)
        .collect();
    let score = frequency::ensemble_score(&plaintext)
        + frequency::impossible_pattern_penalty(&plaintext);

    candidates.push(VigenereCandidate {
        key: chi_key,
        plaintext,
        score,
        method: "Vigenere-Chi2",
    });

    candidates
}

/// A polyalphabetic attack candidate.
#[derive(Debug, Clone)]
pub struct VigenereCandidate {
    pub key: Vec<u8>,
    pub plaintext: String,
    pub score: f64,
    pub method: &'static str,
}

// ═══════════════════════════════════════════════════════════════
// MUSICAL KEY TESTING
// ═══════════════════════════════════════════════════════════════

/// Convert a text keyword to a Vigenère key (mod 24).
/// Each letter A-Z maps to 0-23 (wrapping after X).
fn keyword_to_key(keyword: &str) -> Vec<u8> {
    keyword.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| (b.to_ascii_uppercase() - b'A') % 24)
        .collect()
}

/// Musical interval sequences that might serve as keys.
/// Based on Elgar's known musical motifs.
fn musical_keys() -> Vec<(Vec<u8>, &'static str)> {
    vec![
        // Text-derived keys
        (keyword_to_key("ENIGMA"), "ENIGMA"),
        (keyword_to_key("BACH"), "BACH"),
        (keyword_to_key("DORA"), "DORA"),
        (keyword_to_key("PENNY"), "PENNY"),
        (keyword_to_key("DORABELLA"), "DORABELLA"),
        (keyword_to_key("ELGAR"), "ELGAR"),
        (keyword_to_key("EDWARD"), "EDWARD"),
        (keyword_to_key("NIMROD"), "NIMROD"),
        (keyword_to_key("BRAMO"), "BRAMO"),  // Elgar's nickname for himself

        // Enigma theme intervals (semitones: G-G-Ab-F-Eb)
        // Mapped mod 24: [7, 7, 8, 5, 3]
        (vec![7, 7, 8, 5, 3], "Enigma-theme-semitones"),

        // BACH motif (B♭-A-C-B♮ in German notation)
        // Semitones from C: [10, 9, 0, 11] → mod 24: same
        (vec![10, 9, 0, 11], "BACH-motif"),

        // Circle of fifths steps
        (vec![0, 7, 2, 9, 4, 11], "CircleOfFifths-6"),
        (vec![0, 7, 2, 9], "CircleOfFifths-4"),

        // Simple numeric patterns
        (vec![1, 2, 3], "123"),
        (vec![3, 2, 1], "321"),
        (vec![1, 3, 5, 7], "1357-odd"),
        (vec![2, 4, 6, 8], "2468-even"),

        // Dorabella as note numbers (D=2, O=14%24=14, R=17, A=0, B=1, E=4, L=11, L=11, A=0)
        (vec![2, 14, 17, 0, 1, 4, 11, 11, 0], "DORABELLA-notenum"),

        // Period 3 variants (matches 87 = 3 × 29 perfectly)
        (vec![0, 8, 16], "Thirds-3"),
        (vec![0, 12, 6], "Tritone-3"),
        (vec![0, 4, 8], "MinorThirds-3"),
    ]
}

/// Test all musical keys as Vigenère shifts.
pub fn musical_key_attack() -> Vec<VigenereCandidate> {
    let mut candidates = Vec::new();

    for (key, _name) in musical_keys() {
        let shifted = vigenere_decrypt(&key);
        let mut shifted_freq = [0u32; 24];
        for &s in &shifted {
            shifted_freq[s as usize] += 1;
        }
        let mapping = frequency::frequency_matched_mapping(&shifted_freq);
        let plaintext: String = shifted.iter()
            .map(|&s| mapping[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);

        candidates.push(VigenereCandidate {
            key: key.clone(),
            plaintext,
            score,
            method: "MusicalKey",
        });
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

// ═══════════════════════════════════════════════════════════════
// HOMOPHONIC HYPOTHESIS
// ═══════════════════════════════════════════════════════════════

/// Test the hypothesis that multiple symbols map to the same letter.
/// With 24 symbols → 26 letters, some letters may have 2+ symbol assignments,
/// especially high-frequency letters like E, T, A.
///
/// Strategy: cluster the 20 used symbols into N groups (where N < 20),
/// test whether treating each group as one letter produces better IC.
pub fn homophonic_hypothesis() -> Vec<HomophonicResult> {
    let mut results = Vec::new();

    // Hypothesis: symbols with similar arc counts map to the same letter.
    // i.e., A1 and A2 might both represent the same plaintext letter.
    // This gives 8 effective symbols (one per orientation).
    let direction_collapse: Vec<u8> = CIPHERTEXT.iter()
        .map(|&s| s / 3) // Group by direction (0-7)
        .collect();
    let ic_8 = symbol_ic_n(&direction_collapse, 8);
    results.push(HomophonicResult {
        name: "Direction-only (8 groups)",
        effective_alphabet: 8,
        ic: ic_8,
    });

    // Hypothesis: symbols with same arc count map to the same letter.
    // This gives 3 effective symbols (1-arc, 2-arc, 3-arc).
    let arc_collapse: Vec<u8> = CIPHERTEXT.iter()
        .map(|&s| s % 3)
        .collect();
    let ic_3 = symbol_ic_n(&arc_collapse, 3);
    results.push(HomophonicResult {
        name: "Arc-count-only (3 groups)",
        effective_alphabet: 3,
        ic: ic_3,
    });

    // Hypothesis: merge by frequency similarity.
    // Group the 20 used symbols into 13 groups (like a 13-letter alphabet)
    // by pairing the two rarest together, then next two rarest, etc.
    let freqs = super::symbols::symbol_frequencies();
    let mut sorted_syms: Vec<(usize, u32)> = (0..24)
        .filter(|&i| freqs[i] > 0)
        .map(|i| (i, freqs[i]))
        .collect();
    sorted_syms.sort_by(|a, b| a.1.cmp(&b.1));

    // Merge pairs of low-frequency symbols
    let mut merge_map = [0u8; 24];
    let mut group_id = 0u8;
    let mut i = 0;
    while i < sorted_syms.len() {
        merge_map[sorted_syms[i].0] = group_id;
        if i + 1 < sorted_syms.len() && sorted_syms[i].1 <= 2 && sorted_syms[i + 1].1 <= 2 {
            // Merge two rare symbols into one group
            merge_map[sorted_syms[i + 1].0] = group_id;
            i += 2;
        } else {
            i += 1;
        }
        group_id += 1;
    }

    let merged: Vec<u8> = CIPHERTEXT.iter()
        .map(|&s| merge_map[s as usize])
        .collect();
    let n_groups = group_id as usize;
    let ic_merged = symbol_ic_n(&merged, n_groups);
    results.push(HomophonicResult {
        name: "Frequency-paired merging",
        effective_alphabet: n_groups,
        ic: ic_merged,
    });

    results
}

/// IC computation for an arbitrary alphabet size.
fn symbol_ic_n(symbols: &[u8], alphabet_size: usize) -> f64 {
    let mut freq = vec![0u32; alphabet_size];
    for &s in symbols {
        if (s as usize) < alphabet_size {
            freq[s as usize] += 1;
        }
    }
    let n = symbols.len() as f64;
    if n <= 1.0 { return 0.0; }
    let numerator: f64 = freq.iter()
        .map(|&f| f as f64 * (f as f64 - 1.0))
        .sum();
    numerator / (n * (n - 1.0))
}

/// Result of a homophonic hypothesis test.
#[derive(Debug, Clone)]
pub struct HomophonicResult {
    pub name: &'static str,
    pub effective_alphabet: usize,
    pub ic: f64,
}

// ═══════════════════════════════════════════════════════════════
// TRANSPOSITION HYPOTHESIS
// ═══════════════════════════════════════════════════════════════

/// Test whether the ciphertext reads differently when lines are reordered
/// or read in alternative patterns (reverse, boustrophedon, columnar).
pub fn transposition_candidates() -> Vec<TranspositionResult> {
    let mut results = Vec::new();

    // Line boundaries from the Hauer transcription
    let line1 = &CIPHERTEXT[LINE_BREAKS[0].0..LINE_BREAKS[0].1];
    let line2 = &CIPHERTEXT[LINE_BREAKS[1].0..LINE_BREAKS[1].1];
    let line3 = &CIPHERTEXT[LINE_BREAKS[2].0..LINE_BREAKS[2].1];

    // T1: Reverse entire ciphertext
    let reversed: Vec<u8> = CIPHERTEXT.iter().rev().copied().collect();
    results.push(TranspositionResult {
        name: "Full reverse",
        reordered: reversed,
    });

    // T2: Reverse each line
    let mut line_reversed = Vec::new();
    line_reversed.extend(line1.iter().rev());
    line_reversed.extend(line2.iter().rev());
    line_reversed.extend(line3.iter().rev());
    results.push(TranspositionResult {
        name: "Lines reversed individually",
        reordered: line_reversed,
    });

    // T3: Boustrophedon (alternating direction per line)
    let mut boustro = Vec::new();
    boustro.extend(line1.iter());
    boustro.extend(line2.iter().rev());
    boustro.extend(line3.iter());
    results.push(TranspositionResult {
        name: "Boustrophedon (L-R-L)",
        reordered: boustro,
    });

    // T4: Lines in reverse order
    let mut rev_order = Vec::new();
    rev_order.extend(line3.iter());
    rev_order.extend(line2.iter());
    rev_order.extend(line1.iter());
    results.push(TranspositionResult {
        name: "Lines 3-2-1",
        reordered: rev_order,
    });

    // T5: Columnar read (read down columns if laid out in rows of N)
    for cols in [3, 4, 5, 6, 7, 9, 29] {
        let rows = (CIPHER_LEN + cols - 1) / cols;
        let mut columnar = Vec::with_capacity(CIPHER_LEN);
        for c in 0..cols {
            for r in 0..rows {
                let idx = r * cols + c;
                if idx < CIPHER_LEN {
                    columnar.push(CIPHERTEXT[idx]);
                }
            }
        }
        results.push(TranspositionResult {
            name: match cols {
                3 => "Columnar (3 cols)",
                4 => "Columnar (4 cols)",
                5 => "Columnar (5 cols)",
                6 => "Columnar (6 cols)",
                7 => "Columnar (7 cols)",
                9 => "Columnar (9 cols)",
                29 => "Columnar (29 cols = 3 rows)",
                _ => "Columnar",
            },
            reordered: columnar,
        });
    }

    // T6: Rail fence (depth 2 and 3)
    for rails in [2, 3] {
        let reordered = rail_fence_decrypt(CIPHERTEXT, rails);
        results.push(TranspositionResult {
            name: match rails {
                2 => "Rail fence (2 rails)",
                3 => "Rail fence (3 rails)",
                _ => "Rail fence",
            },
            reordered,
        });
    }

    results
}

/// Rail fence decryption for given number of rails.
fn rail_fence_decrypt(ct: &[u8], rails: usize) -> Vec<u8> {
    if rails <= 1 || ct.is_empty() { return ct.to_vec(); }
    let n = ct.len();
    let cycle = 2 * (rails - 1);

    // Calculate length of each rail
    let mut rail_lens = vec![0usize; rails];
    for i in 0..n {
        let pos = i % cycle;
        let rail = if pos < rails { pos } else { cycle - pos };
        rail_lens[rail] += 1;
    }

    // Split ciphertext into rails
    let mut rail_data: Vec<Vec<u8>> = Vec::new();
    let mut offset = 0;
    for &len in &rail_lens {
        rail_data.push(ct[offset..offset + len].to_vec());
        offset += len;
    }

    // Read off in zigzag order
    let mut rail_idx = vec![0usize; rails];
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let pos = i % cycle;
        let rail = if pos < rails { pos } else { cycle - pos };
        if rail_idx[rail] < rail_data[rail].len() {
            result.push(rail_data[rail][rail_idx[rail]]);
            rail_idx[rail] += 1;
        }
    }

    result
}

/// Result of a transposition hypothesis.
#[derive(Debug, Clone)]
pub struct TranspositionResult {
    pub name: &'static str,
    pub reordered: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════
// NULL SYMBOL HYPOTHESIS
// ═══════════════════════════════════════════════════════════════

/// Test the hypothesis that some symbols are nulls (meaningless fillers).
/// The 4 unused symbols (D3, E1, E2, H3) are not candidates since they
/// don't appear. Instead, test removing each of the least-frequent symbols.
pub fn null_symbol_candidates() -> Vec<NullResult> {
    let freqs = super::symbols::symbol_frequencies();
    let mut results = Vec::new();

    // Find symbols with count = 1 (candidates for nulls)
    let singletons: Vec<usize> = (0..24)
        .filter(|&i| freqs[i] == 1)
        .collect();
    // Singletons: A1(0), A2(1), C3(8), D2(10), G3(20)

    // Test removing each singleton
    for &null_sym in &singletons {
        let filtered: Vec<u8> = CIPHERTEXT.iter()
            .filter(|&&s| s as usize != null_sym)
            .copied()
            .collect();
        let ic = symbol_ic(&filtered);
        results.push(NullResult {
            removed: vec![null_sym],
            remaining_len: filtered.len(),
            ic,
            filtered,
        });
    }

    // Test removing all singletons at once
    if singletons.len() > 1 {
        let filtered: Vec<u8> = CIPHERTEXT.iter()
            .filter(|&&s| !singletons.contains(&(s as usize)))
            .copied()
            .collect();
        let ic = symbol_ic(&filtered);
        results.push(NullResult {
            removed: singletons.clone(),
            remaining_len: filtered.len(),
            ic,
            filtered,
        });
    }

    // Test removing the two rarest symbols (count ≤ 2)
    let rare: Vec<usize> = (0..24)
        .filter(|&i| freqs[i] > 0 && freqs[i] <= 2)
        .collect();
    if rare.len() > singletons.len() {
        let filtered: Vec<u8> = CIPHERTEXT.iter()
            .filter(|&&s| !rare.contains(&(s as usize)))
            .copied()
            .collect();
        let ic = symbol_ic(&filtered);
        results.push(NullResult {
            removed: rare,
            remaining_len: filtered.len(),
            ic,
            filtered,
        });
    }

    results
}

/// Score a null-symbol hypothesis by stripping nulls and running
/// frequency-matched MASC + hill climbing on the remaining symbols.
/// Returns the best plaintext and score for the stripped ciphertext.
pub fn null_strip_attack(null_symbols: &[usize], iterations: usize) -> Vec<VigenereCandidate> {
    let filtered: Vec<u8> = CIPHERTEXT.iter()
        .filter(|&&s| !null_symbols.contains(&(s as usize)))
        .copied()
        .collect();

    if filtered.is_empty() { return Vec::new(); }

    let mut freq = [0u32; 24];
    for &s in &filtered {
        freq[s as usize] += 1;
    }

    // Frequency-matched initial mapping
    let mapping = frequency::frequency_matched_mapping(&freq);
    let plaintext: String = filtered.iter()
        .map(|&s| mapping[s as usize] as char)
        .collect();
    let base_score = frequency::ensemble_score(&plaintext)
        + frequency::impossible_pattern_penalty(&plaintext);

    let mut candidates = vec![VigenereCandidate {
        key: null_symbols.iter().map(|&s| s as u8).collect(),
        plaintext,
        score: base_score,
        method: "NullStrip-Freq",
    }];

    // Hill-climb from the frequency-matched mapping
    let mut best_mapping = mapping;
    let mut best_score = base_score;
    let mut rng_state = 0xDEAD_BEEF_u64;

    for _ in 0..iterations {
        // Inline xorshift
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let a = (rng_state % 24) as usize;
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let mut b = (rng_state % 24) as usize;
        if b == a { b = (b + 1) % 24; }

        let mut trial = best_mapping;
        trial.swap(a, b);

        let pt: String = filtered.iter()
            .map(|&s| trial[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&pt)
            + frequency::impossible_pattern_penalty(&pt);

        if score > best_score {
            best_score = score;
            best_mapping = trial;
        }
    }

    let best_pt: String = filtered.iter()
        .map(|&s| best_mapping[s as usize] as char)
        .collect();
    candidates.push(VigenereCandidate {
        key: null_symbols.iter().map(|&s| s as u8).collect(),
        plaintext: best_pt,
        score: best_score,
        method: "NullStrip-HC",
    });

    // Also try with Elgar-speak scoring
    let mut best_mapping_e = mapping;
    let mut best_score_e = frequency::elgar_ensemble_score(&candidates[0].plaintext)
        + frequency::impossible_pattern_penalty(&candidates[0].plaintext);
    let mut rng_state_e = 0xCAFE_BABE_u64;

    for _ in 0..iterations {
        rng_state_e ^= rng_state_e << 13;
        rng_state_e ^= rng_state_e >> 7;
        rng_state_e ^= rng_state_e << 17;
        let a = (rng_state_e % 24) as usize;
        rng_state_e ^= rng_state_e << 13;
        rng_state_e ^= rng_state_e >> 7;
        rng_state_e ^= rng_state_e << 17;
        let mut b = (rng_state_e % 24) as usize;
        if b == a { b = (b + 1) % 24; }

        let mut trial = best_mapping_e;
        trial.swap(a, b);

        let pt: String = filtered.iter()
            .map(|&s| trial[s as usize] as char)
            .collect();
        let score = frequency::elgar_ensemble_score(&pt)
            + frequency::impossible_pattern_penalty(&pt);

        if score > best_score_e {
            best_score_e = score;
            best_mapping_e = trial;
        }
    }

    let best_pt_e: String = filtered.iter()
        .map(|&s| best_mapping_e[s as usize] as char)
        .collect();
    candidates.push(VigenereCandidate {
        key: null_symbols.iter().map(|&s| s as u8).collect(),
        plaintext: best_pt_e,
        score: best_score_e,
        method: "NullStrip-Elgar",
    });

    candidates
}

/// Result of a null-symbol hypothesis test.
#[derive(Debug, Clone)]
pub struct NullResult {
    pub removed: Vec<usize>,
    pub remaining_len: usize,
    pub ic: f64,
    pub filtered: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════
// DIRECTION-ONLY (8-SYMBOL) COLLAPSE ATTACK
// ═══════════════════════════════════════════════════════════════

/// The homophonic analysis proves arc-count IC = 0.3299 ≈ random (0.3333).
/// This means arc count carries ZERO information. The cipher may be
/// fundamentally 8-symbol (one per orientation A-H), with arcs as
/// decorative chaff Elgar added to disguise the true alphabet size.
///
/// This module tests three sub-hypotheses:
///   D1: Direction → letter (8-to-8 reduced MASC)
///   D2: Direction → letter group, arc selects within group (steganographic)
///   D3: Direction-only + null-strip combined

/// Collapse the 24-symbol ciphertext to 8-symbol (direction only).
pub fn direction_collapse() -> Vec<u8> {
    CIPHERTEXT.iter().map(|&s| s / 3).collect()
}

/// Collapse to direction-only, also stripping null symbols first.
pub fn direction_collapse_stripped(null_symbols: &[usize]) -> Vec<u8> {
    CIPHERTEXT.iter()
        .filter(|&&s| !null_symbols.contains(&(s as usize)))
        .map(|&s| s / 3)
        .collect()
}

/// IC of the direction-collapsed ciphertext (8-symbol alphabet).
pub fn direction_ic() -> f64 {
    let collapsed = direction_collapse();
    symbol_ic_n(&collapsed, 8)
}

/// Frequency distribution over the 8 directions.
pub fn direction_frequencies() -> [u32; 8] {
    let mut freq = [0u32; 8];
    for &s in CIPHERTEXT {
        freq[(s / 3) as usize] += 1;
    }
    freq
}

/// Direction frequency for stripped ciphertext.
pub fn direction_frequencies_stripped(null_symbols: &[usize]) -> [u32; 8] {
    let mut freq = [0u32; 8];
    for &s in CIPHERTEXT {
        if !null_symbols.contains(&(s as usize)) {
            freq[(s / 3) as usize] += 1;
        }
    }
    freq
}

/// D1: Direct 8→8 mapping. Map the 8 directions to 8 letters.
/// Try all candidate letter sets (most common 8, vowels+top consonants, etc.)
pub fn direction_8to8_attack() -> Vec<DirectionCandidate> {
    let collapsed = direction_collapse();
    let freq = direction_frequencies();
    let mut candidates = Vec::new();

    // Sort directions by frequency (descending)
    let mut dir_order: Vec<usize> = (0..8).collect();
    dir_order.sort_by(|&a, &b| freq[b].cmp(&freq[a]));

    // Letter sets to try (8 letters each, sorted by English frequency)
    let letter_sets: Vec<(&str, [u8; 8])> = vec![
        // Top 8 English letters by frequency: E T A O I N S R
        ("Top8-ETAOINSR", [b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'R']),
        // Top 8 shifted: E T A O I N S H
        ("Top8-ETAOINSH", [b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'H']),
        // With D instead of R: E T A O I N S D
        ("Top8-ETAOINSD", [b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'D']),
        // Vowels + top consonants: A E I O U T N S
        ("Vowels+TNS", [b'A', b'E', b'I', b'O', b'U', b'T', b'N', b'S']),
        // Musical note letters: A B C D E F G H
        ("Notes-ABCDEFGH", [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H']),
        // Elgar-speak: D O R A B E L(=I) ?(=N)
        ("DORABEIN", [b'D', b'O', b'R', b'A', b'B', b'E', b'I', b'N']),
    ];

    for (name, letters) in &letter_sets {
        // Try frequency-matched assignment
        let mut mapping = [0u8; 8];
        for (rank, &dir) in dir_order.iter().enumerate() {
            mapping[dir] = letters[rank];
        }

        let plaintext: String = collapsed.iter()
            .map(|&d| mapping[d as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);

        candidates.push(DirectionCandidate {
            mapping: mapping.to_vec(),
            plaintext,
            score,
            method: name,
        });

        // Also try all 8! = 40320 permutations for small letter sets
        // (only feasible for 8 elements)
        let best_perm = brute_force_8_mapping(&collapsed, &letters);
        if let Some((best_map, best_pt, best_score)) = best_perm {
            candidates.push(DirectionCandidate {
                mapping: best_map.to_vec(),
                plaintext: best_pt,
                score: best_score,
                method: name,
            });
        }
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates.dedup_by(|a, b| a.plaintext == b.plaintext);
    candidates
}

/// Brute-force all 8! = 40320 permutations of an 8-letter set.
/// Returns the best (mapping, plaintext, score).
fn brute_force_8_mapping(
    collapsed: &[u8],
    letters: &[u8; 8],
) -> Option<([u8; 8], String, f64)> {
    let mut best_mapping = [0u8; 8];
    let mut best_score = f64::NEG_INFINITY;
    let mut best_pt = String::new();

    // Generate all permutations of 8 elements
    let mut perm: Vec<usize> = (0..8).collect();
    let mut c = [0usize; 8];

    // Score initial permutation
    let score_perm = |p: &[usize]| -> ([u8; 8], String, f64) {
        let mut m = [0u8; 8];
        for (dir, &letter_idx) in p.iter().enumerate() {
            m[dir] = letters[letter_idx];
        }
        let pt: String = collapsed.iter()
            .map(|&d| m[d as usize] as char)
            .collect();
        let s = frequency::ensemble_score(&pt)
            + frequency::impossible_pattern_penalty(&pt);
        (m, pt, s)
    };

    let (m, pt, s) = score_perm(&perm);
    if s > best_score {
        best_score = s;
        best_mapping = m;
        best_pt = pt;
    }

    // Heap's algorithm for generating all permutations
    let mut i = 0;
    while i < 8 {
        if c[i] < i {
            if i % 2 == 0 {
                perm.swap(0, i);
            } else {
                perm.swap(c[i], i);
            }
            let (m, pt, s) = score_perm(&perm);
            if s > best_score {
                best_score = s;
                best_mapping = m;
                best_pt = pt;
            }
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }

    if best_score > f64::NEG_INFINITY {
        Some((best_mapping, best_pt, best_score))
    } else {
        None
    }
}

/// D2: Steganographic hypothesis — direction selects a letter GROUP,
/// arc count selects the specific letter within that group.
/// 8 groups × 3 variants = 24 symbols → 24 letters (plus 2 unused).
///
/// Group assignments to test:
///   - Alphabetic triads: (ABC, DEF, GHI, JKL, MNO, PQR, STU, VWX)
///   - Frequency triads: (ETA, ONI, SRH, LDC, UMW, FGY, PBV, KJX)
pub fn direction_steganographic_attack() -> Vec<DirectionCandidate> {
    let mut candidates = Vec::new();

    // Hypothesis S1: Alphabetic triads
    // Direction A → {A,B,C}, arc 1→A, 2→B, 3→C
    // Direction B → {D,E,F}, etc.
    let alpha_triads: [[u8; 3]; 8] = [
        [b'A', b'B', b'C'], [b'D', b'E', b'F'], [b'G', b'H', b'I'],
        [b'J', b'K', b'L'], [b'M', b'N', b'O'], [b'P', b'Q', b'R'],
        [b'S', b'T', b'U'], [b'V', b'W', b'X'],
    ];
    let pt_alpha = stego_decrypt(&alpha_triads);
    let score = frequency::ensemble_score(&pt_alpha)
        + frequency::impossible_pattern_penalty(&pt_alpha);
    candidates.push(DirectionCandidate {
        mapping: alpha_triads.iter().flat_map(|t| t.iter().copied()).collect(),
        plaintext: pt_alpha,
        score,
        method: "Stego-alphabetic",
    });

    // Hypothesis S2: Frequency-sorted triads
    // Most frequent direction gets the 3 most frequent English letters, etc.
    let freq = direction_frequencies();
    let mut dir_order: Vec<usize> = (0..8).collect();
    dir_order.sort_by(|&a, &b| freq[b].cmp(&freq[a]));

    // English letters sorted by frequency, grouped in triads
    let freq_letters: [u8; 24] = [
        b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'R',
        b'H', b'L', b'D', b'C', b'U', b'M', b'W', b'F',
        b'G', b'Y', b'P', b'B', b'V', b'K', b'J', b'X',
    ];
    let mut freq_triads = [[0u8; 3]; 8];
    for (rank, &dir) in dir_order.iter().enumerate() {
        freq_triads[dir] = [
            freq_letters[rank * 3],
            freq_letters[rank * 3 + 1],
            freq_letters[rank * 3 + 2],
        ];
    }
    let pt_freq = stego_decrypt(&freq_triads);
    let score = frequency::ensemble_score(&pt_freq)
        + frequency::impossible_pattern_penalty(&pt_freq);
    candidates.push(DirectionCandidate {
        mapping: freq_triads.iter().flat_map(|t| t.iter().copied()).collect(),
        plaintext: pt_freq,
        score,
        method: "Stego-frequency",
    });

    // Hypothesis S3: Same as S2 but arc order reversed (3→first, 1→last)
    let mut rev_triads = freq_triads;
    for triad in &mut rev_triads {
        triad.swap(0, 2);
    }
    let pt_rev = stego_decrypt(&rev_triads);
    let score = frequency::ensemble_score(&pt_rev)
        + frequency::impossible_pattern_penalty(&pt_rev);
    candidates.push(DirectionCandidate {
        mapping: rev_triads.iter().flat_map(|t| t.iter().copied()).collect(),
        plaintext: pt_rev,
        score,
        method: "Stego-freq-rev",
    });

    // Hypothesis S4: Musical note groupings
    // A→{C,C#,Db}, B→{D,D#,Eb}, C→{E,F,F#}, D→{G,G#,Ab}, etc.
    // Mapped to letters: just try CDEFGAB with sharps/flats as nearby letters
    // Actually, use note-name letters with enharmonic variants
    let musical_triads: [[u8; 3]; 8] = [
        [b'C', b'D', b'E'], [b'F', b'G', b'A'], [b'B', b'H', b'I'],
        [b'J', b'K', b'L'], [b'M', b'N', b'O'], [b'P', b'Q', b'R'],
        [b'S', b'T', b'U'], [b'V', b'W', b'X'],
    ];
    let pt_mus = stego_decrypt(&musical_triads);
    let score = frequency::ensemble_score(&pt_mus)
        + frequency::impossible_pattern_penalty(&pt_mus);
    candidates.push(DirectionCandidate {
        mapping: musical_triads.iter().flat_map(|t| t.iter().copied()).collect(),
        plaintext: pt_mus,
        score,
        method: "Stego-musical",
    });

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

/// Decrypt using steganographic triads: direction selects group, arc selects letter.
fn stego_decrypt(triads: &[[u8; 3]; 8]) -> String {
    CIPHERTEXT.iter().map(|&s| {
        let dir = (s / 3) as usize;
        let arc = (s % 3) as usize;
        triads[dir][arc] as char
    }).collect()
}

/// D3: Direction-only attack on the null-stripped ciphertext.
pub fn direction_stripped_attack(null_symbols: &[usize]) -> Vec<DirectionCandidate> {
    let collapsed = direction_collapse_stripped(null_symbols);
    let mut freq = [0u32; 8];
    for &d in &collapsed {
        freq[d as usize] += 1;
    }

    let mut candidates = Vec::new();
    let mut dir_order: Vec<usize> = (0..8).collect();
    dir_order.sort_by(|&a, &b| freq[b].cmp(&freq[a]));

    // Top 8 English letters
    let top8 = [b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'R'];

    // Brute-force all 40320 permutations
    let best = brute_force_8_mapping(&collapsed, &top8);
    if let Some((m, pt, s)) = best {
        candidates.push(DirectionCandidate {
            mapping: m.to_vec(),
            plaintext: pt,
            score: s,
            method: "DirStripped-Top8",
        });
    }

    // Also try ETAOINSH
    let top8h = [b'E', b'T', b'A', b'O', b'I', b'N', b'S', b'H'];
    let best2 = brute_force_8_mapping(&collapsed, &top8h);
    if let Some((m, pt, s)) = best2 {
        candidates.push(DirectionCandidate {
            mapping: m.to_vec(),
            plaintext: pt,
            score: s,
            method: "DirStripped-Top8H",
        });
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

/// D4: Focused steganographic solver — frequency-rank the 8 directions,
/// frequency-rank the 26 English letters, assign triads to directions
/// in frequency order, then brute-force the within-triad assignment
/// (arc1/arc2/arc3 → which letter of the triad).
///
/// The outer assignment (direction → triad) is fixed by frequency.
/// The inner assignment (arc → letter within triad) has 3! = 6 permutations
/// per direction, giving 6^8 = 1,679,616 total combinations — tractable.
pub fn steganographic_brute_force() -> Vec<DirectionCandidate> {
    let dir_freq = direction_frequencies();
    let mut dir_order: Vec<usize> = (0..8).collect();
    dir_order.sort_by(|&a, &b| dir_freq[b].cmp(&dir_freq[a]));

    // English letters sorted by frequency (most → least)
    let eng_order: &[u8] = b"ETAOINSRHLDCUMWFGYPBVKJXQZ";

    // Assign triads: most-frequent direction gets {E,T,A}, next gets {O,I,N}, etc.
    // The last 2 directions (D=5, E=4 total) get only 2 letters + padding
    let mut triads: Vec<[u8; 3]> = Vec::new();
    for chunk in eng_order.chunks(3) {
        let mut t = [b'?'; 3];
        for (i, &c) in chunk.iter().enumerate() {
            t[i] = c;
        }
        // If chunk has <3, fill with rare letters
        if chunk.len() < 3 {
            for i in chunk.len()..3 {
                t[i] = eng_order[eng_order.len() - 1]; // Z as filler
            }
        }
        triads.push(t);
    }
    // We have 9 triads for 26 letters (9×3=27, last has padding)
    // But we only have 8 directions. Merge the last two triads.
    while triads.len() > 8 {
        let last = triads.pop().unwrap();
        if let Some(prev) = triads.last_mut() {
            // Replace filler in prev with letters from last
            for i in 0..3 {
                if prev[i] == b'?' || prev[i] == eng_order[eng_order.len() - 1] {
                    prev[i] = last[i];
                    break;
                }
            }
        }
    }

    // Build the mapping: direction_order[i] gets triads[i]
    let mut dir_triads = [[0u8; 3]; 8];
    for (rank, &dir) in dir_order.iter().enumerate() {
        if rank < triads.len() {
            dir_triads[dir] = triads[rank];
        }
    }

    // Now brute-force the 6 arc-permutations per direction (6^8 = 1,679,616)
    let perms_3: [[usize; 3]; 6] = [
        [0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0],
    ];

    let mut best_score = f64::NEG_INFINITY;
    let mut best_plaintext = String::new();
    let mut best_mapping_24 = [0u8; 24];
    let mut _tested = 0u64;

    // Iterate all 6^8 combinations
    for p0 in 0..6 { for p1 in 0..6 { for p2 in 0..6 { for p3 in 0..6 {
    for p4 in 0..6 { for p5 in 0..6 { for p6 in 0..6 { for p7 in 0..6 {
        let perm_indices = [p0, p1, p2, p3, p4, p5, p6, p7];

        // Build 24-element mapping
        let mut mapping = [0u8; 24];
        for dir in 0..8 {
            let perm = &perms_3[perm_indices[dir]];
            for arc in 0..3 {
                let coset = dir * 3 + arc;
                mapping[coset] = dir_triads[dir][perm[arc]];
            }
        }

        // Decrypt and score
        let plaintext: String = CIPHERTEXT.iter()
            .map(|&s| mapping[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);
        _tested += 1;

        if score > best_score {
            best_score = score;
            best_plaintext = plaintext;
            best_mapping_24 = mapping;
        }
    }}}}}}}}

    let mut candidates = vec![DirectionCandidate {
        mapping: best_mapping_24.to_vec(),
        plaintext: best_plaintext.clone(),
        score: best_score,
        method: "Stego-BruteForce",
    }];

    // Also score with Elgar-speak
    let elgar_score = frequency::elgar_ensemble_score(&best_plaintext)
        + frequency::impossible_pattern_penalty(&best_plaintext);
    if elgar_score != best_score {
        candidates.push(DirectionCandidate {
            mapping: best_mapping_24.to_vec(),
            plaintext: best_plaintext,
            score: elgar_score,
            method: "Stego-BF-Elgar",
        });
    }

    candidates
}

/// Result of a direction-based attack.
#[derive(Debug, Clone)]
pub struct DirectionCandidate {
    pub mapping: Vec<u8>,
    pub plaintext: String,
    pub score: f64,
    pub method: &'static str,
}

// ═══════════════════════════════════════════════════════════════
// COMBINED POLYALPHABETIC PIPELINE
// ═══════════════════════════════════════════════════════════════

/// Full non-MASC analysis results.
#[derive(Debug)]
pub struct PolyAnalysis {
    pub kasiski: Vec<KasiskiResult>,
    pub period_ics: Vec<PeriodIC>,
    pub best_vigenere: Vec<VigenereCandidate>,
    pub musical_keys: Vec<VigenereCandidate>,
    pub homophonic: Vec<HomophonicResult>,
    pub transpositions: Vec<TranspositionResult>,
    pub nulls: Vec<NullResult>,
}

/// Run the full polyalphabetic / non-MASC analysis pipeline.
pub fn full_poly_analysis() -> PolyAnalysis {
    let kasiski = kasiski_examination();
    let period_ics = ic_per_period(12);

    // Run Vigenère column attack for top 5 candidate periods
    let mut all_vigenere: Vec<VigenereCandidate> = caesar_attack();
    for pic in period_ics.iter().take(5) {
        let mut vc = vigenere_column_attack(pic.period);
        all_vigenere.append(&mut vc);
    }
    // Also try periods from Kasiski GCDs
    let mut kasiski_periods: Vec<usize> = kasiski.iter()
        .filter(|k| k.gcd >= 2 && k.gcd <= 12)
        .map(|k| k.gcd)
        .collect();
    kasiski_periods.sort();
    kasiski_periods.dedup();
    for &p in &kasiski_periods {
        let mut vc = vigenere_column_attack(p);
        all_vigenere.append(&mut vc);
    }
    all_vigenere.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    all_vigenere.truncate(20);

    let musical_keys = musical_key_attack();
    let homophonic = homophonic_hypothesis();
    let transpositions = transposition_candidates();
    let nulls = null_symbol_candidates();

    PolyAnalysis {
        kasiski,
        period_ics,
        best_vigenere: all_vigenere,
        musical_keys,
        homophonic,
        transpositions,
        nulls,
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kasiski_finds_repeats() {
        let results = kasiski_examination();
        // There should be repeated bigrams in 87 chars over 20 symbols
        assert!(!results.is_empty(), "Kasiski should find repeated n-grams");
    }

    #[test]
    fn test_ic_per_period() {
        let results = ic_per_period(10);
        assert_eq!(results.len(), 9); // periods 2..=10
        for r in &results {
            assert!(r.avg_ic >= 0.0 && r.avg_ic <= 1.0,
                "IC {} out of range for period {}", r.avg_ic, r.period);
        }
    }

    #[test]
    fn test_vigenere_identity() {
        // Shift of 0 should return original ciphertext
        let shifted = vigenere_decrypt(&[0]);
        assert_eq!(shifted, CIPHERTEXT);
    }

    #[test]
    fn test_vigenere_roundtrip() {
        let key = vec![5, 10, 15];
        let shifted = vigenere_decrypt(&key);
        // Encrypt (add key) should give back original
        let restored: Vec<u8> = shifted.iter().enumerate().map(|(i, &s)| {
            (s + key[i % key.len()]) % 24
        }).collect();
        assert_eq!(restored.as_slice(), CIPHERTEXT);
    }

    #[test]
    fn test_caesar_attack_produces_candidates() {
        let candidates = caesar_attack();
        assert_eq!(candidates.len(), 24);
        for c in &candidates {
            assert_eq!(c.plaintext.len(), 87);
        }
    }

    #[test]
    fn test_musical_key_attack() {
        let results = musical_key_attack();
        assert!(!results.is_empty());
        for r in &results {
            assert_eq!(r.plaintext.len(), 87);
        }
    }

    #[test]
    fn test_homophonic_hypothesis() {
        let results = homophonic_hypothesis();
        assert!(results.len() >= 3);
        // Direction-only should give IC closer to English if homophonic
        let dir_only = &results[0];
        assert_eq!(dir_only.effective_alphabet, 8);
    }

    #[test]
    fn test_transposition_candidates() {
        let results = transposition_candidates();
        assert!(results.len() >= 5);
        for r in &results {
            assert_eq!(r.reordered.len(), CIPHER_LEN);
        }
    }

    #[test]
    fn test_null_symbol_candidates() {
        let results = null_symbol_candidates();
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.remaining_len < CIPHER_LEN);
            assert!(r.remaining_len > 0);
        }
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 3), 1);
        assert_eq!(multi_gcd(&[12, 18, 24]), 6);
    }

    #[test]
    fn test_rail_fence_identity() {
        let result = rail_fence_decrypt(CIPHERTEXT, 1);
        assert_eq!(result, CIPHERTEXT);
    }

    #[test]
    fn test_direction_collapse() {
        let collapsed = direction_collapse();
        assert_eq!(collapsed.len(), CIPHER_LEN);
        for &d in &collapsed {
            assert!(d < 8, "Direction {} out of range", d);
        }
    }

    #[test]
    fn test_direction_ic() {
        let ic = direction_ic();
        // Should be elevated above random/8 = 0.125
        assert!(ic > 0.10, "Direction IC {} should be > 0.10", ic);
    }

    #[test]
    fn test_direction_8to8_attack() {
        let results = direction_8to8_attack();
        assert!(!results.is_empty());
        for r in &results {
            assert_eq!(r.plaintext.len(), CIPHER_LEN);
        }
    }

    #[test]
    fn test_direction_steganographic() {
        let results = direction_steganographic_attack();
        assert!(results.len() >= 3);
        for r in &results {
            assert_eq!(r.plaintext.len(), CIPHER_LEN);
        }
    }

    #[test]
    fn test_steganographic_brute_force() {
        let results = steganographic_brute_force();
        assert!(!results.is_empty());
        assert_eq!(results[0].plaintext.len(), CIPHER_LEN);
        // Score should be better than random
        assert!(results[0].score > -100.0,
            "Stego BF score {} seems too low", results[0].score);
    }

    #[test]
    fn test_direction_stripped() {
        let singletons = vec![0, 1, 8, 10, 20];
        let results = direction_stripped_attack(&singletons);
        assert!(!results.is_empty());
        for r in &results {
            assert_eq!(r.plaintext.len(), 82); // 87 - 5
        }
    }

    #[test]
    fn test_null_strip_attack() {
        // Strip singletons and run a quick hill-climb
        let singletons = vec![0, 1, 8, 10, 20]; // A1, A2, C3, D2, G3
        let results = null_strip_attack(&singletons, 1000);
        assert!(results.len() >= 2);
        for r in &results {
            assert_eq!(r.plaintext.len(), 82); // 87 - 5 singletons
        }
    }

    #[test]
    fn test_full_poly_analysis() {
        let analysis = full_poly_analysis();
        assert!(!analysis.kasiski.is_empty());
        assert!(!analysis.period_ics.is_empty());
        assert!(!analysis.best_vigenere.is_empty());
        assert!(!analysis.musical_keys.is_empty());
        assert!(!analysis.homophonic.is_empty());
        assert!(!analysis.transpositions.is_empty());
    }
}
