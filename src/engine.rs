/// Dorabella Cipher — 8-Phase Attack Engine
///
/// Orchestrates multiple cryptanalysis strategies against Elgar's 87-character
/// cipher, exploiting the 24-coset ↔ 24-symbol structural isomorphism.
///
/// Phase 1: Frequency-matched initial mapping
/// Phase 2: Hill-climbing with bigram/trigram scoring
/// Phase 3: Simulated annealing over substitution space
/// Phase 4: Genetic algorithm (OX crossover)
/// Phase 5: Crib dragging (contextual plaintext fragments)
/// Phase 6: Basin clustering via coset dynamics
/// Phase 7: Spectral resonance scoring
/// Phase 8: Musical hypothesis branch (see musical.rs)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use rayon::prelude::*;

use super::symbols::{self, CIPHERTEXT, CIPHER_LEN, CONTEXTUAL_CRIBS};
use super::frequency;
use super::musical;
use super::vigenere;

// ═══════════════════════════════════════════════════════════════
// XORSHIFT64 PRNG (inline, fast non-crypto RNG)
// ═══════════════════════════════════════════════════════════════

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0xDEADBEEF_CAFEBABE } else { seed })
    }

    fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234567890ABCDEF);
        Rng::new(nanos)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn usize(&mut self, max: usize) -> usize {
        if max == 0 { return 0; }
        (self.next() % max as u64) as usize
    }

    fn f64(&mut self) -> f64 {
        (self.next() as f64) / (u64::MAX as f64)
    }
}

// ═══════════════════════════════════════════════════════════════
// ATTACK CONFIGURATION & RESULTS
// ═══════════════════════════════════════════════════════════════

/// Configuration for the Dorabella attack.
#[derive(Debug, Clone)]
pub struct DorabellaConfig {
    /// Maximum seconds for hill-climbing phase.
    pub hill_climb_secs: u64,
    /// Maximum seconds for simulated annealing phase.
    pub anneal_secs: u64,
    /// Number of hill-climb restarts.
    pub hill_climb_restarts: usize,
    /// Annealing initial temperature.
    pub anneal_temp: f64,
    /// Annealing cooling rate (0.9999 = slow, 0.999 = fast).
    pub anneal_cooling: f64,
    /// Genetic algorithm population size.
    pub genetic_pop: usize,
    /// Genetic algorithm generations.
    pub genetic_gens: usize,
    /// Number of parallel threads (0 = auto).
    pub threads: usize,
    /// Run musical hypothesis branch.
    pub try_musical: bool,
    /// Overall timeout in seconds (0 = unlimited).
    pub timeout_secs: u64,
}

impl Default for DorabellaConfig {
    fn default() -> Self {
        DorabellaConfig {
            hill_climb_secs: 30,
            anneal_secs: 60,
            hill_climb_restarts: 50,
            anneal_temp: 20.0,
            anneal_cooling: 0.9999,
            genetic_pop: 500,
            genetic_gens: 5000,
            threads: 0,
            try_musical: true,
            timeout_secs: 300,
        }
    }
}

/// A single candidate solution.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Mapping from coset index (0..24) to ASCII letter (A-Z).
    pub mapping: [u8; 24],
    /// Decrypted plaintext.
    pub plaintext: String,
    /// Ensemble score (higher = more English-like).
    pub score: f64,
    /// Which phase produced this candidate.
    pub phase: &'static str,
}

/// Result of the full attack.
#[derive(Debug)]
pub struct DorabellaResult {
    /// Top candidates, sorted by score (best first).
    pub candidates: Vec<Candidate>,
    /// Total mappings evaluated.
    pub mappings_tested: u64,
    /// Wall-clock time in milliseconds.
    pub elapsed_ms: u128,
    /// Phase-by-phase breakdown.
    pub phase_results: Vec<PhaseResult>,
}

/// Per-phase result summary.
#[derive(Debug)]
pub struct PhaseResult {
    pub phase: &'static str,
    pub best_score: f64,
    pub best_plaintext: String,
    pub mappings_tested: u64,
    pub elapsed_ms: u128,
}

// ═══════════════════════════════════════════════════════════════
// MAPPING UTILITIES
// ═══════════════════════════════════════════════════════════════

/// Decrypt the ciphertext using a given mapping.
fn decrypt(mapping: &[u8; 24]) -> String {
    symbols::decrypt_with_mapping(mapping)
}

/// Score a mapping using the ensemble scorer.
fn score_mapping(mapping: &[u8; 24]) -> f64 {
    let pt = decrypt(mapping);
    frequency::ensemble_score(&pt) + frequency::impossible_pattern_penalty(&pt)
}

/// Swap two random positions in a mapping.
fn swap_mapping(mapping: &mut [u8; 24], rng: &mut Rng) {
    let a = rng.usize(24);
    let mut b = rng.usize(24);
    while b == a { b = rng.usize(24); }
    mapping.swap(a, b);
}

/// Create a random mapping (random permutation of A-Z, first 24).
fn random_mapping(rng: &mut Rng) -> [u8; 24] {
    let mut letters: Vec<u8> = (b'A'..=b'Z').collect();
    // Fisher-Yates shuffle
    for i in (1..letters.len()).rev() {
        let j = rng.usize(i + 1);
        letters.swap(i, j);
    }
    let mut mapping = [0u8; 24];
    for i in 0..24 {
        mapping[i] = letters[i];
    }
    mapping
}

// ═══════════════════════════════════════════════════════════════
// PHASE 1: FREQUENCY-MATCHED INITIAL MAPPING
// ═══════════════════════════════════════════════════════════════

fn phase1_frequency(tested: &AtomicU64) -> Candidate {
    let freqs = symbols::symbol_frequencies();
    let mapping = frequency::frequency_matched_mapping(&freqs);
    tested.fetch_add(1, Ordering::Relaxed);
    let plaintext = decrypt(&mapping);
    let score = frequency::ensemble_score(&plaintext)
        + frequency::impossible_pattern_penalty(&plaintext);
    Candidate { mapping, plaintext, score, phase: "P1:Frequency" }
}

// ═══════════════════════════════════════════════════════════════
// PHASE 2: HILL CLIMBING
// ═══════════════════════════════════════════════════════════════

fn phase2_hill_climb(
    config: &DorabellaConfig,
    tested: &AtomicU64,
    found: &AtomicBool,
    start: &Instant,
) -> Vec<Candidate> {
    let deadline = config.hill_climb_secs;

    // Run multiple restarts in parallel
    let results: Vec<Candidate> = (0..config.hill_climb_restarts)
        .into_par_iter()
        .filter_map(|restart_id| {
            if found.load(Ordering::Relaxed) { return None; }
            if deadline > 0 && start.elapsed().as_secs() > deadline { return None; }

            let mut rng = Rng::new(restart_id as u64 * 7919 + 42);
            let mut mapping = if restart_id == 0 {
                // First restart: start from frequency match
                let freqs = symbols::symbol_frequencies();
                frequency::frequency_matched_mapping(&freqs)
            } else {
                random_mapping(&mut rng)
            };
            let mut best_score = score_mapping(&mapping);
            let mut best_mapping = mapping;
            tested.fetch_add(1, Ordering::Relaxed);

            // Hill-climb: swap two positions, keep if better
            let max_iters = 50_000;
            let mut stagnant = 0u32;
            for _ in 0..max_iters {
                if deadline > 0 && start.elapsed().as_secs() > deadline { break; }

                let mut candidate = mapping;
                swap_mapping(&mut candidate, &mut rng);
                let s = score_mapping(&candidate);
                tested.fetch_add(1, Ordering::Relaxed);

                if s > best_score {
                    best_score = s;
                    best_mapping = candidate;
                    mapping = candidate;
                    stagnant = 0;
                } else {
                    stagnant += 1;
                    if stagnant > 5000 { break; } // Converged
                }
            }

            let plaintext = decrypt(&best_mapping);
            Some(Candidate {
                mapping: best_mapping,
                plaintext,
                score: best_score,
                phase: "P2:HillClimb",
            })
        })
        .collect();

    results
}

// ═══════════════════════════════════════════════════════════════
// PHASE 3: SIMULATED ANNEALING
// ═══════════════════════════════════════════════════════════════

fn phase3_anneal(
    config: &DorabellaConfig,
    seed_mapping: &[u8; 24],
    tested: &AtomicU64,
    start: &Instant,
) -> Candidate {
    let mut rng = Rng::from_time();
    let mut mapping = *seed_mapping;
    let mut score = score_mapping(&mapping);
    let mut best_mapping = mapping;
    let mut best_score = score;
    let mut temp = config.anneal_temp;
    tested.fetch_add(1, Ordering::Relaxed);

    let max_iters = 500_000;
    for _ in 0..max_iters {
        if config.anneal_secs > 0 && start.elapsed().as_secs() > config.anneal_secs + config.hill_climb_secs {
            break;
        }
        if temp < 0.001 { break; }

        let mut candidate = mapping;
        swap_mapping(&mut candidate, &mut rng);
        let new_score = score_mapping(&candidate);
        tested.fetch_add(1, Ordering::Relaxed);

        let delta = new_score - score;
        // Accept if better, or probabilistically if worse (SA acceptance)
        if delta > 0.0 || rng.f64() < (delta / temp).exp() {
            mapping = candidate;
            score = new_score;
            if score > best_score {
                best_score = score;
                best_mapping = mapping;
            }
        }

        temp *= config.anneal_cooling;
    }

    let plaintext = decrypt(&best_mapping);
    Candidate {
        mapping: best_mapping,
        plaintext,
        score: best_score,
        phase: "P3:Anneal",
    }
}

// ═══════════════════════════════════════════════════════════════
// PHASE 4: GENETIC ALGORITHM
// ═══════════════════════════════════════════════════════════════

fn phase4_genetic(
    config: &DorabellaConfig,
    seed_mapping: &[u8; 24],
    tested: &AtomicU64,
    start: &Instant,
) -> Candidate {
    let pop_size = config.genetic_pop;
    let max_gens = config.genetic_gens;
    let mut rng = Rng::from_time();

    // Initialize population
    let mut population: Vec<([u8; 24], f64)> = Vec::with_capacity(pop_size);
    // Seed with best known
    population.push((*seed_mapping, score_mapping(seed_mapping)));
    tested.fetch_add(1, Ordering::Relaxed);

    // Fill rest with random + frequency-seeded variants
    while population.len() < pop_size {
        let mapping = random_mapping(&mut rng);
        let s = score_mapping(&mapping);
        tested.fetch_add(1, Ordering::Relaxed);
        population.push((mapping, s));
    }

    let elite_count = (pop_size as f64 * 0.05).max(1.0) as usize;
    let mut best_mapping = seed_mapping.clone();
    let mut best_score = population[0].1;

    for gen in 0..max_gens {
        if config.timeout_secs > 0 && start.elapsed().as_secs() > config.timeout_secs {
            break;
        }

        // Sort by score (descending = best first)
        population.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Track best
        if population[0].1 > best_score {
            best_score = population[0].1;
            best_mapping = population[0].0;
        }

        // Elitism
        let mut next_gen: Vec<([u8; 24], f64)> = population[..elite_count].to_vec();

        // Generate offspring
        while next_gen.len() < pop_size {
            // Tournament selection (size 3)
            let parent_a = tournament_select(&population, 3, &mut rng);
            let parent_b = tournament_select(&population, 3, &mut rng);

            // Crossover: order crossover (OX) for permutation mappings
            let mut child = order_crossover(&parent_a, &parent_b, &mut rng);

            // Mutation: swap two positions (30% rate)
            if rng.f64() < 0.3 {
                swap_mapping(&mut child, &mut rng);
            }

            // Adaptive mutation: extra swaps every 500 gens
            if gen > 0 && gen % 500 == 0 && rng.f64() < 0.5 {
                swap_mapping(&mut child, &mut rng);
                swap_mapping(&mut child, &mut rng);
            }

            let s = score_mapping(&child);
            tested.fetch_add(1, Ordering::Relaxed);
            next_gen.push((child, s));
        }

        // Inject random immigrants every 200 gens (prevent stagnation)
        if gen > 0 && gen % 200 == 0 {
            let inject = pop_size / 10;
            for i in 0..inject {
                let idx = pop_size - 1 - i;
                if idx < next_gen.len() {
                    let m = random_mapping(&mut rng);
                    let s = score_mapping(&m);
                    tested.fetch_add(1, Ordering::Relaxed);
                    next_gen[idx] = (m, s);
                }
            }
        }

        next_gen.truncate(pop_size);
        population = next_gen;
    }

    let plaintext = decrypt(&best_mapping);
    Candidate {
        mapping: best_mapping,
        plaintext,
        score: best_score,
        phase: "P4:Genetic",
    }
}

/// Tournament selection for genetic phase.
fn tournament_select(pop: &[([u8; 24], f64)], size: usize, rng: &mut Rng) -> [u8; 24] {
    let mut best_idx = rng.usize(pop.len());
    for _ in 1..size {
        let idx = rng.usize(pop.len());
        if pop[idx].1 > pop[best_idx].1 {
            best_idx = idx;
        }
    }
    pop[best_idx].0
}

/// Order crossover (OX) for permutation-based mappings.
/// Preserves the permutation property (no duplicate letters).
fn order_crossover(parent_a: &[u8; 24], parent_b: &[u8; 24], rng: &mut Rng) -> [u8; 24] {
    let mut start = rng.usize(24);
    let mut end = rng.usize(24);
    if start > end { std::mem::swap(&mut start, &mut end); }
    if start == end { end = (end + 1) % 24; if start > end { std::mem::swap(&mut start, &mut end); } }

    let mut child = [0u8; 24];
    let mut used = [false; 26]; // track which letters are placed

    // Copy segment from parent A
    for i in start..=end.min(23) {
        child[i] = parent_a[i];
        used[(parent_a[i] - b'A') as usize] = true;
    }

    // Collect remaining letters from parent B (in order, excluding used)
    let remaining: Vec<u8> = parent_b.iter()
        .filter(|&&l| !used[(l - b'A') as usize])
        .copied()
        .collect();

    let mut ri = 0;
    for i in 0..24 {
        if i >= start && i <= end.min(23) { continue; }
        if ri < remaining.len() {
            child[i] = remaining[ri];
            used[(remaining[ri] - b'A') as usize] = true;
            ri += 1;
        }
    }

    // Fill any gaps (shouldn't happen, but safety)
    for i in 0..24 {
        if child[i] == 0 {
            for l in b'A'..=b'Z' {
                if !used[(l - b'A') as usize] {
                    child[i] = l;
                    used[(l - b'A') as usize] = true;
                    break;
                }
            }
        }
    }

    child
}

// ═══════════════════════════════════════════════════════════════
// PHASE 5: CRIB DRAGGING
// ═══════════════════════════════════════════════════════════════

/// Try placing known words at every position in the ciphertext
/// and build partial mappings, then complete with frequency matching.
fn phase5_crib_drag(tested: &AtomicU64) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let freqs = symbols::symbol_frequencies();

    for crib in CONTEXTUAL_CRIBS {
        let crib_bytes: Vec<u8> = crib.bytes()
            .filter(|b| b.is_ascii_alphabetic())
            .map(|b| b.to_ascii_uppercase())
            .collect();

        if crib_bytes.len() > CIPHER_LEN { continue; }

        // Try placing crib at each valid position
        for pos in 0..=(CIPHER_LEN - crib_bytes.len()) {
            // Check consistency: same ciphertext symbol must map to same letter
            let mut partial = [0u8; 24];
            let mut used_letters = [false; 26];
            let mut consistent = true;

            for (i, &letter) in crib_bytes.iter().enumerate() {
                let sym = CIPHERTEXT[pos + i] as usize;
                let letter_idx = (letter - b'A') as usize;

                if partial[sym] == 0 {
                    // Not yet assigned
                    if used_letters[letter_idx] {
                        // Letter already used by a different symbol
                        consistent = false;
                        break;
                    }
                    partial[sym] = letter;
                    used_letters[letter_idx] = true;
                } else if partial[sym] != letter {
                    // Conflict: same symbol must map to same letter
                    consistent = false;
                    break;
                }
            }

            if !consistent { continue; }

            // Complete the mapping with frequency matching for unassigned symbols
            let mut mapping = partial;
            let mut sym_by_freq: Vec<usize> = (0..24)
                .filter(|&i| mapping[i] == 0)
                .collect();
            sym_by_freq.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));

            let mut remaining_letters: Vec<u8> = (b'A'..=b'Z')
                .filter(|l| !used_letters[(*l - b'A') as usize])
                .collect();
            // Sort remaining by English frequency (descending)
            remaining_letters.sort_by(|a, b| {
                let fa = frequency::ENGLISH_FREQ[(*a - b'A') as usize];
                let fb = frequency::ENGLISH_FREQ[(*b - b'A') as usize];
                fb.partial_cmp(&fa).unwrap()
            });

            for (idx, &sym) in sym_by_freq.iter().enumerate() {
                if idx < remaining_letters.len() {
                    mapping[sym] = remaining_letters[idx];
                } else {
                    mapping[sym] = b'X'; // Fallback
                }
            }

            tested.fetch_add(1, Ordering::Relaxed);
            let plaintext = decrypt(&mapping);
            let score = frequency::ensemble_score(&plaintext)
                + frequency::impossible_pattern_penalty(&plaintext);

            candidates.push(Candidate {
                mapping,
                plaintext,
                score,
                phase: "P5:CribDrag",
            });
        }
    }

    // Sort by score, keep top results
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates.truncate(100);
    candidates
}

// ═══════════════════════════════════════════════════════════════
// PHASE 6: BASIN CLUSTERING
// ═══════════════════════════════════════════════════════════════

/// Cluster the 24 symbols into groups based on the 4 orientation basins
/// and test vowel/consonant assignments per cluster.
fn phase6_basin_cluster(tested: &AtomicU64) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let freqs = symbols::symbol_frequencies();

    // The 4 orientation basins (grouping by direction pairs):
    //   Basin 0 (Creation):   cosets {0,1,2,3,4,5}     → orientations A,B
    //   Basin 1 (Perception): cosets {6,7,8,9,10,11}    → orientations C,D
    //   Basin 2 (Stability):  cosets {12,13,14,15,16,17} → orientations E,F
    //   Basin 3 (Exchange):   cosets {18,19,20,21,22,23} → orientations G,H
    let basins: [&[usize]; 4] = [
        &[0, 1, 2, 3, 4, 5],
        &[6, 7, 8, 9, 10, 11],
        &[12, 13, 14, 15, 16, 17],
        &[18, 19, 20, 21, 22, 23],
    ];

    // Hypothesis: vowels cluster in high-frequency basins,
    // consonants in lower-frequency basins.
    let vowels = b"AEIOU";
    let consonants = b"BCDFGHJKLMNPQRSTVWXYZ";

    // Compute basin-level total frequencies
    let mut basin_freqs: Vec<(usize, u32)> = basins.iter().enumerate()
        .map(|(bi, cosets)| {
            let total: u32 = cosets.iter().map(|&c| freqs[c]).sum();
            (bi, total)
        })
        .collect();
    basin_freqs.sort_by(|a, b| b.1.cmp(&a.1));

    // Try assigning vowels to the two highest-frequency basins
    let high_basins = [basin_freqs[0].0, basin_freqs[1].0];
    let _low_basins = [basin_freqs[2].0, basin_freqs[3].0];

    // For each high basin, assign vowels by intra-basin frequency
    let mut rng = Rng::new(0xBA51_0001);
    for _ in 0..20 { // 20 random basin-consistent assignments
        let mut mapping = [0u8; 24];
        let mut used = [false; 26];

        // Assign vowels to high-frequency basins
        let mut vowel_pool: Vec<u8> = vowels.to_vec();
        for &bi in &high_basins {
            let mut cosets: Vec<usize> = basins[bi].to_vec();
            cosets.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));
            for &c in &cosets {
                if !vowel_pool.is_empty() {
                    let vi = rng.usize(vowel_pool.len());
                    let v = vowel_pool.remove(vi);
                    mapping[c] = v;
                    used[(v - b'A') as usize] = true;
                }
            }
        }

        // Assign consonants to low-frequency basins + remaining slots
        let mut cons_pool: Vec<u8> = consonants.iter()
            .filter(|&&c| !used[(c - b'A') as usize])
            .copied()
            .collect();
        // Sort by English frequency
        cons_pool.sort_by(|a, b| {
            let fa = frequency::ENGLISH_FREQ[(*a - b'A') as usize];
            let fb = frequency::ENGLISH_FREQ[(*b - b'A') as usize];
            fb.partial_cmp(&fa).unwrap()
        });

        let mut unassigned: Vec<usize> = (0..24)
            .filter(|&i| mapping[i] == 0)
            .collect();
        unassigned.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));

        for (idx, &c) in unassigned.iter().enumerate() {
            if idx < cons_pool.len() {
                mapping[c] = cons_pool[idx];
            } else {
                // Use remaining letters
                for l in b'A'..=b'Z' {
                    if !used[(l - b'A') as usize] && mapping.iter().all(|&m| m != l) {
                        mapping[c] = l;
                        break;
                    }
                }
            }
        }

        tested.fetch_add(1, Ordering::Relaxed);
        let plaintext = decrypt(&mapping);
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);

        candidates.push(Candidate { mapping, plaintext, score, phase: "P6:Basin" });
    }

    candidates
}

// ═══════════════════════════════════════════════════════════════
// PHASE 7: SPECTRAL RESONANCE SCORING
// ═══════════════════════════════════════════════════════════════

/// Uses spectral decomposition to test whether a mapping
/// produces plaintext that "resonates" with expected frequency structure.
///
/// The hypothesis: correct decryption creates text whose letter-frequency
/// vector aligns with the expected English frequency distribution.
fn phase7_spectral_refine(
    best_candidates: &[Candidate],
    tested: &AtomicU64,
    start: &Instant,
    timeout: u64,
) -> Vec<Candidate> {
    let mut refined = Vec::new();
    let mut rng = Rng::new(0x5EC7_BA01);

    for candidate in best_candidates.iter().take(10) {
        if timeout > 0 && start.elapsed().as_secs() > timeout { break; }

        // Spectral refinement: hill-climb from the candidate
        // using frequency rank correlation as an additional signal
        let mut mapping = candidate.mapping;
        let mut best_score = candidate.score;

        for _ in 0..10_000 {
            let mut trial = mapping;
            swap_mapping(&mut trial, &mut rng);
            let pt = decrypt(&trial);
            let base = frequency::ensemble_score(&pt)
                + frequency::impossible_pattern_penalty(&pt);

            // Spectral bonus: reward mappings where plaintext IC
            // is close to English IC (0.0667)
            let ic = frequency::text_ic(&pt);
            let ic_bonus = -((ic - 0.0667).abs() * 50.0);

            // Frequency rank correlation bonus
            let rank_corr = frequency::frequency_rank_correlation(&pt);
            let rank_bonus = rank_corr * 5.0;

            let total = base + ic_bonus + rank_bonus;
            tested.fetch_add(1, Ordering::Relaxed);

            if total > best_score {
                best_score = total;
                mapping = trial;
            }
        }

        let plaintext = decrypt(&mapping);
        refined.push(Candidate {
            mapping,
            plaintext,
            score: best_score,
            phase: "P7:Spectral",
        });
    }

    refined
}

// ═══════════════════════════════════════════════════════════════
// PHASE 9: VIGENÈRE / POLYALPHABETIC
// ═══════════════════════════════════════════════════════════════

/// Run the polyalphabetic attack pipeline from vigenere.rs.
/// Tests Kasiski-derived periods, IC-optimal periods, musical keys,
/// and transposition hypotheses.
fn phase9_vigenere(tested: &AtomicU64) -> Vec<Candidate> {
    let analysis = vigenere::full_poly_analysis();
    let mut candidates = Vec::new();

    // Convert Vigenère candidates to engine Candidates
    for vc in analysis.best_vigenere.iter().take(10) {
        tested.fetch_add(1, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&vc.plaintext),
            plaintext: vc.plaintext.clone(),
            score: vc.score,
            phase: "P9:Vigenere",
        });
    }

    // Musical key candidates
    for vc in analysis.musical_keys.iter().take(10) {
        tested.fetch_add(1, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&vc.plaintext),
            plaintext: vc.plaintext.clone(),
            score: vc.score,
            phase: "P9:MusicalKey",
        });
    }

    // Transposition + MASC: apply each transposition, then frequency-match
    for tr in &analysis.transpositions {
        let mut freq = [0u32; 24];
        for &s in &tr.reordered {
            freq[s as usize] += 1;
        }
        let mapping = frequency::frequency_matched_mapping(&freq);
        let plaintext: String = tr.reordered.iter()
            .map(|&s| mapping[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);
        tested.fetch_add(1, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping,
            plaintext,
            score,
            phase: "P9:Transpose",
        });
    }

    // Null-symbol + MASC: remove nulls, then frequency-match
    for nr in &analysis.nulls {
        let mut freq = [0u32; 24];
        for &s in &nr.filtered {
            freq[s as usize] += 1;
        }
        let mapping = frequency::frequency_matched_mapping(&freq);
        let plaintext: String = nr.filtered.iter()
            .map(|&s| mapping[s as usize] as char)
            .collect();
        let score = frequency::ensemble_score(&plaintext)
            + frequency::impossible_pattern_penalty(&plaintext);
        tested.fetch_add(1, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping,
            plaintext,
            score,
            phase: "P9:NullRemove",
        });
    }

    // Null-strip + hill climbing: the strongest null hypothesis
    // (remove all singletons → IC approaches English)
    let singletons: Vec<usize> = {
        let freqs = symbols::symbol_frequencies();
        (0..24).filter(|&i| freqs[i] == 1).collect()
    };
    let rare: Vec<usize> = {
        let freqs = symbols::symbol_frequencies();
        (0..24).filter(|&i| freqs[i] > 0 && freqs[i] <= 2).collect()
    };
    let singletons_copy = singletons.clone();
    for null_set in &[singletons_copy, rare] {
        let null_results = vigenere::null_strip_attack(null_set, 50_000);
        for vc in null_results {
            tested.fetch_add(50_000, Ordering::Relaxed);
            candidates.push(Candidate {
                mapping: build_mapping_from_plaintext(&vc.plaintext),
                plaintext: vc.plaintext,
                score: vc.score,
                phase: "P9:NullStrip",
            });
        }
    }

    // Direction-only (8-symbol) attacks
    // Arc-count IC = random → direction is sole information carrier
    let dir_8to8 = vigenere::direction_8to8_attack();
    for dc in dir_8to8.iter().take(10) {
        tested.fetch_add(40320, Ordering::Relaxed); // 8! permutations per letter set
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&dc.plaintext),
            plaintext: dc.plaintext.clone(),
            score: dc.score,
            phase: "P9:Dir8to8",
        });
    }

    // Steganographic: direction=group, arc=selector
    let stego = vigenere::direction_steganographic_attack();
    for dc in stego.iter().take(5) {
        tested.fetch_add(1, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&dc.plaintext),
            plaintext: dc.plaintext.clone(),
            score: dc.score,
            phase: "P9:DirStego",
        });
    }

    // Direction-only on stripped ciphertext
    let dir_stripped = vigenere::direction_stripped_attack(&singletons);
    for dc in dir_stripped.iter().take(5) {
        tested.fetch_add(40320, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&dc.plaintext),
            plaintext: dc.plaintext.clone(),
            score: dc.score,
            phase: "P9:DirStrip",
        });
    }

    // Steganographic brute-force: 6^8 = 1.68M arc-permutation combos
    let stego_bf = vigenere::steganographic_brute_force();
    for dc in stego_bf.iter().take(5) {
        tested.fetch_add(1_679_616, Ordering::Relaxed);
        candidates.push(Candidate {
            mapping: build_mapping_from_plaintext(&dc.plaintext),
            plaintext: dc.plaintext.clone(),
            score: dc.score,
            phase: "P9:StegoBF",
        });
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates.truncate(30);
    candidates
}

/// Build a best-guess mapping array from plaintext (reverse-engineer).
/// This is approximate — maps each cipher symbol to the most common
/// letter it produced in the given plaintext.
fn build_mapping_from_plaintext(plaintext: &str) -> [u8; 24] {
    let bytes: Vec<u8> = plaintext.bytes().collect();
    let mut mapping = [b'?'; 24];
    let mut used = [false; 26];

    // For each cipher symbol, find what letter appears most often
    for sym in 0..24u8 {
        let mut counts = [0u32; 26];
        for (i, &s) in CIPHERTEXT.iter().enumerate() {
            if s == sym && i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                counts[(bytes[i].to_ascii_uppercase() - b'A') as usize] += 1;
            }
        }
        // Find most frequent letter not yet used
        let mut best_letter = b'?';
        let mut best_count = 0;
        for l in 0..26 {
            if counts[l] > best_count && !used[l] {
                best_count = counts[l];
                best_letter = b'A' + l as u8;
            }
        }
        if best_letter != b'?' {
            mapping[sym as usize] = best_letter;
            used[(best_letter - b'A') as usize] = true;
        }
    }

    // Fill unmapped symbols
    for sym in 0..24 {
        if mapping[sym] == b'?' {
            for l in 0..26u8 {
                if !used[l as usize] {
                    mapping[sym] = b'A' + l;
                    used[l as usize] = true;
                    break;
                }
            }
        }
    }
    mapping
}

// ═══════════════════════════════════════════════════════════════
// PHASE 10: ELGAR-SPEAK HILL CLIMBING
// ═══════════════════════════════════════════════════════════════

/// Hill-climb using the Elgar-speak scoring model instead of standard
/// English. This rewards backslang, musical terms, abbreviations, and
/// Victorian letter-writing patterns that Elgar was known to use.
fn phase10_elgar_speak(
    seed_mappings: &[[u8; 24]],
    config: &DorabellaConfig,
    tested: &AtomicU64,
    start: &Instant,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    for seed in seed_mappings {
        if config.timeout_secs > 0 && start.elapsed().as_secs() > config.timeout_secs {
            break;
        }

        let mut rng = Rng::from_time();
        let mut mapping = *seed;
        let pt = decrypt(&mapping);
        let score = frequency::elgar_ensemble_score(&pt)
            + frequency::impossible_pattern_penalty(&pt);
        let mut best_mapping = mapping;
        let mut best_score = score;
        tested.fetch_add(1, Ordering::Relaxed);

        // Hill-climb with Elgar-speak scoring
        let iters = 20_000;
        let mut stagnant = 0u32;
        for _ in 0..iters {
            if config.timeout_secs > 0 && start.elapsed().as_secs() > config.timeout_secs {
                break;
            }

            let mut trial = mapping;
            swap_mapping(&mut trial, &mut rng);
            let pt = decrypt(&trial);
            let s = frequency::elgar_ensemble_score(&pt)
                + frequency::impossible_pattern_penalty(&pt);
            tested.fetch_add(1, Ordering::Relaxed);

            if s > best_score {
                best_score = s;
                best_mapping = trial;
                mapping = trial;
                stagnant = 0;
            } else {
                stagnant += 1;
                if stagnant > 5000 { break; }
            }
        }

        let plaintext = decrypt(&best_mapping);
        candidates.push(Candidate {
            mapping: best_mapping,
            plaintext,
            score: best_score,
            phase: "P10:ElgarSpeak",
        });
    }

    candidates
}

// ═══════════════════════════════════════════════════════════════
// PHASE 11: CRIB-PINNED MUSICAL MESSAGE ATTACK
// ═══════════════════════════════════════════════════════════════

/// The stable mapping core — converged identically across 7M, 25M, and 103M runs.
/// These symbol→letter assignments are treated as ground truth.
const PINNED_MAPPING: [(usize, u8); 16] = [
    (15, b'T'),  // F1 → T  (THEME)
    ( 3, b'H'),  // B1 → H  (THEME, YOUR-adjacent)
    (16, b'E'),  // F2 → E  (most frequent: 11 occurrences)
    ( 8, b'M'),  // C3 → M  (THEME)
    ( 7, b'N'),  // C2 → N
    (21, b'O'),  // H1 → O  (YOUR)
    ( 6, b'U'),  // C1 → U  (YOUR)
    ( 5, b'R'),  // B3 → R
    ( 4, b'S'),  // B2 → S
    ( 2, b'A'),  // A3 → A
    (19, b'D'),  // G2 → D
    (18, b'G'),  // G1 → G
    ( 9, b'F'),  // D1 → F  (FOR)
    (17, b'I'),  // F3 → I  (IS IN)
    (22, b'Y'),  // H2 → Y  (YOUR — stable across all runs)
    (14, b'L'),  // E3 → L  (stable across all runs)
];

/// Musical-message cribs based on the hypothesis that Elgar was telling Dora
/// about a theme he was composing for her (the future Enigma Variation X).
const MUSICAL_MESSAGE_CRIBS: &[&str] = &[
    // Top-priority cribs (verified against stable mapping)
    "THETHEMEISFORYOU",
    "THETHEMEISFOR",
    "THETHEMEISYOURS",
    "IHAVEYOURTHEME",
    "IHAVEMADEYOURTHEME",
    "YOURTHEME",
    "THETHEME",
    "THEMEFORYOU",
    "THEMEFORDORABELLA",
    "THEMEFORTISH",
    "FOROURTHEME",
    // Composer's confession phrases
    "IHAVEATHEMEFORYOU",
    "IHAVEWROTEATHEMEFORYOU",
    "IWROTETHEMEFORYOU",
    "IWROTEATHEMEFORYOU",
    "IHAVEMADEATHEME",
    "AMAKINGATHEMEFORYOU",
    "HEREISYOURTHEME",
    "HEREYOURTHEME",
    "MYTHEMEFORYOU",
    "ATHEMEFORYOU",
    "ATHEMEIWROTEFOR",
    "THISISYOURTHEME",
    "ITISYOURTHEME",
    "ABOUTYOURTHEME",
    "YOUOWNTHETHEME",
    // Victorian openers + name
    "MYDEARDORABELLA",
    "MYDEARDORA",
    "MYDEARTISH",
    "DEARESTDORA",
    "DEARESTTISH",
    "MYDEARFRIEND",
    // Musical composition references
    "THETHEMEISINYOUR",
    "THEMELODYISFOR",
    "THISMELODYISFORYOU",
    "THETUNEISFORYOU",
    "ENIGMATHEME",
    "ENIGMAFORYOU",
    "THEVARIATIONISFOR",
    "AVARIATIONFORYOU",
    // Elgar-speak / personal
    "FORYOUTISH",
    "TISHMYMELODY",
    "TISHYOURMELODY",
    "THISISYOURS",
    "ITISYOURS",
    "YOUALONE",
    // Contextual fragments
    "ISINYOURNAME",
    "ISFOREVER",
    "ISINTHEMUSIC",
    "WRITTENFOR",
    "COMPOSEDFOR",
    "MUSICFORYOU",
];

/// Swap only unpinned positions.
fn swap_unpinned(mapping: &mut [u8; 24], pinned: &[bool; 24], rng: &mut Rng) {
    let unpinned: Vec<usize> = (0..24).filter(|&i| !pinned[i]).collect();
    if unpinned.len() < 2 { return; }
    let ai = rng.usize(unpinned.len());
    let mut bi = rng.usize(unpinned.len());
    while bi == ai { bi = rng.usize(unpinned.len()); }
    mapping.swap(unpinned[ai], unpinned[bi]);
}

/// Phase 11: Pin the stable core and hill-climb the rest with Elgar-speak scoring.
/// Also tries full-message cribs placed at every valid position.
fn phase11_crib_pinned(
    config: &DorabellaConfig,
    tested: &AtomicU64,
    start: &Instant,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let freqs = symbols::symbol_frequencies();

    // Build pin mask
    let mut pinned = [false; 24];
    let mut base_mapping = [0u8; 24];
    let mut used_letters = [false; 26];

    for &(coset, letter) in &PINNED_MAPPING {
        base_mapping[coset] = letter;
        pinned[coset] = true;
        used_letters[(letter - b'A') as usize] = true;
    }

    // ── Part A: Full-message crib dragging ──────────────────
    for crib in MUSICAL_MESSAGE_CRIBS {
        let crib_bytes: Vec<u8> = crib.bytes()
            .filter(|b| b.is_ascii_alphabetic())
            .map(|b| b.to_ascii_uppercase())
            .collect();
        if crib_bytes.len() > CIPHER_LEN { continue; }

        for pos in 0..=(CIPHER_LEN - crib_bytes.len()) {
            let mut partial = [0u8; 24];
            let mut used = [false; 26];
            let mut consistent = true;

            for (i, &letter) in crib_bytes.iter().enumerate() {
                let sym = CIPHERTEXT[pos + i] as usize;
                let letter_idx = (letter - b'A') as usize;

                if partial[sym] == 0 {
                    if used[letter_idx] {
                        consistent = false;
                        break;
                    }
                    partial[sym] = letter;
                    used[letter_idx] = true;
                } else if partial[sym] != letter {
                    consistent = false;
                    break;
                }
            }

            if !consistent { continue; }

            // Check consistency with pinned mapping
            let mut pin_ok = true;
            for &(coset, letter) in &PINNED_MAPPING {
                if partial[coset] != 0 && partial[coset] != letter {
                    pin_ok = false;
                    break;
                }
            }
            if !pin_ok { continue; }

            // Merge: pinned values override, then crib, then frequency-fill
            let mut mapping = partial;
            let mut all_used = used;
            for &(coset, letter) in &PINNED_MAPPING {
                if mapping[coset] == 0 {
                    mapping[coset] = letter;
                    all_used[(letter - b'A') as usize] = true;
                }
            }

            // Frequency-fill remaining
            let mut unassigned: Vec<usize> = (0..24)
                .filter(|&i| mapping[i] == 0)
                .collect();
            unassigned.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));
            let mut remaining: Vec<u8> = (b'A'..=b'Z')
                .filter(|l| !all_used[(*l - b'A') as usize])
                .collect();
            remaining.sort_by(|a, b| {
                let fa = frequency::ENGLISH_FREQ[(*a - b'A') as usize];
                let fb = frequency::ENGLISH_FREQ[(*b - b'A') as usize];
                fb.partial_cmp(&fa).unwrap()
            });

            for (idx, &sym) in unassigned.iter().enumerate() {
                if idx < remaining.len() {
                    mapping[sym] = remaining[idx];
                } else {
                    mapping[sym] = b'X';
                }
            }

            tested.fetch_add(1, Ordering::Relaxed);
            let plaintext = decrypt(&mapping);
            let score = frequency::elgar_ensemble_score(&plaintext)
                + frequency::impossible_pattern_penalty(&plaintext);
            candidates.push(Candidate {
                mapping, plaintext, score, phase: "P11:CribPin",
            });
        }
    }

    // ── Part B: Pinned hill-climb with Elgar-speak ──────────
    // Start from the pinned base + frequency-fill the rest
    let mut seed_mapping = base_mapping;
    {
        let mut unassigned: Vec<usize> = (0..24)
            .filter(|&i| !pinned[i])
            .collect();
        unassigned.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));
        let mut remaining: Vec<u8> = (b'A'..=b'Z')
            .filter(|l| !used_letters[(*l - b'A') as usize])
            .collect();
        remaining.sort_by(|a, b| {
            let fa = frequency::ENGLISH_FREQ[(*a - b'A') as usize];
            let fb = frequency::ENGLISH_FREQ[(*b - b'A') as usize];
            fb.partial_cmp(&fa).unwrap()
        });
        for (idx, &sym) in unassigned.iter().enumerate() {
            if idx < remaining.len() {
                seed_mapping[sym] = remaining[idx];
            } else {
                seed_mapping[sym] = b'X';
            }
        }
    }

    // Hill-climb: many restarts, swapping only unpinned symbols
    let restarts = 50;
    for restart in 0..restarts {
        if config.timeout_secs > 0 && start.elapsed().as_secs() > config.timeout_secs {
            break;
        }

        let mut rng = Rng::new(0xC01B_0000 + restart as u64);
        let mut mapping = seed_mapping;

        // Randomize unpinned positions for diversity
        if restart > 0 {
            let unpinned: Vec<usize> = (0..24).filter(|&i| !pinned[i]).collect();
            for _ in 0..unpinned.len() * 3 {
                swap_unpinned(&mut mapping, &pinned, &mut rng);
            }
        }

        let pt = decrypt(&mapping);
        let mut best_score = frequency::elgar_ensemble_score(&pt)
            + frequency::impossible_pattern_penalty(&pt);
        let mut best_mapping = mapping;
        tested.fetch_add(1, Ordering::Relaxed);

        // Hill-climb iterations
        let iters = 30_000;
        let mut stagnant = 0u32;
        for _ in 0..iters {
            if config.timeout_secs > 0 && start.elapsed().as_secs() > config.timeout_secs {
                break;
            }

            let mut trial = mapping;
            swap_unpinned(&mut trial, &pinned, &mut rng);
            let pt = decrypt(&trial);
            let s = frequency::elgar_ensemble_score(&pt)
                + frequency::impossible_pattern_penalty(&pt);
            tested.fetch_add(1, Ordering::Relaxed);

            if s > best_score {
                best_score = s;
                best_mapping = trial;
                mapping = trial;
                stagnant = 0;
            } else {
                stagnant += 1;
                if stagnant > 8000 { break; }
            }
        }

        let plaintext = decrypt(&best_mapping);
        candidates.push(Candidate {
            mapping: best_mapping,
            plaintext,
            score: best_score,
            phase: "P11:CribPin",
        });
    }

    // Sort and dedup
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates.dedup_by(|a, b| a.plaintext == b.plaintext);
    candidates.truncate(20);
    candidates
}

// ═══════════════════════════════════════════════════════════════
// MAIN ORCHESTRATOR
// ═══════════════════════════════════════════════════════════════

/// Run the full 8-phase Dorabella attack.
pub fn attack(config: &DorabellaConfig) -> DorabellaResult {
    let start = Instant::now();
    let tested = AtomicU64::new(0);
    let found = AtomicBool::new(false);
    let mut all_candidates: Vec<Candidate> = Vec::new();
    let mut phase_results: Vec<PhaseResult> = Vec::new();

    // ── Phase 1: Frequency Match ──────────────────────────
    let p1_start = Instant::now();
    let p1 = phase1_frequency(&tested);
    phase_results.push(PhaseResult {
        phase: "P1:Frequency",
        best_score: p1.score,
        best_plaintext: p1.plaintext.clone(),
        mappings_tested: 1,
        elapsed_ms: p1_start.elapsed().as_millis(),
    });
    all_candidates.push(p1);

    // ── Phase 2: Hill Climbing ────────────────────────────
    let p2_start = Instant::now();
    let p2_results = phase2_hill_climb(config, &tested, &found, &start);
    let p2_best = p2_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P2:HillClimb",
        best_score: p2_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p2_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p2_start.elapsed().as_millis(),
    });
    all_candidates.extend(p2_results);

    // ── Phase 3: Simulated Annealing ─────────────────────
    let p3_start = Instant::now();
    all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let best_so_far = &all_candidates[0].mapping;
    let p3 = phase3_anneal(config, best_so_far, &tested, &start);
    phase_results.push(PhaseResult {
        phase: "P3:Anneal",
        best_score: p3.score,
        best_plaintext: p3.plaintext.clone(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p3_start.elapsed().as_millis(),
    });
    all_candidates.push(p3);

    // ── Phase 4: Genetic Algorithm ───────────────────────
    let p4_start = Instant::now();
    all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let best_so_far = &all_candidates[0].mapping;
    let p4 = phase4_genetic(config, best_so_far, &tested, &start);
    phase_results.push(PhaseResult {
        phase: "P4:Genetic",
        best_score: p4.score,
        best_plaintext: p4.plaintext.clone(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p4_start.elapsed().as_millis(),
    });
    all_candidates.push(p4);

    // ── Phase 5: Crib Dragging ───────────────────────────
    let p5_start = Instant::now();
    let p5_results = phase5_crib_drag(&tested);
    let p5_best = p5_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P5:CribDrag",
        best_score: p5_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p5_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p5_start.elapsed().as_millis(),
    });
    all_candidates.extend(p5_results);

    // ── Phase 6: Basin Clustering ────────────────────────
    let p6_start = Instant::now();
    let p6_results = phase6_basin_cluster(&tested);
    let p6_best = p6_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P6:Basin",
        best_score: p6_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p6_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p6_start.elapsed().as_millis(),
    });
    all_candidates.extend(p6_results);

    // ── Phase 7: Spectral Refinement ─────────────────────
    let p7_start = Instant::now();
    all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let p7_results = phase7_spectral_refine(
        &all_candidates[..all_candidates.len().min(20)],
        &tested, &start, config.timeout_secs,
    );
    let p7_best = p7_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P7:Spectral",
        best_score: p7_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p7_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p7_start.elapsed().as_millis(),
    });
    all_candidates.extend(p7_results);

    // ── Phase 8: Musical Hypothesis ──────────────────────
    if config.try_musical {
        let p8_start = Instant::now();
        let p8_results = musical::test_musical_hypotheses(&tested);
        let p8_best = p8_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        phase_results.push(PhaseResult {
            phase: "P8:Musical",
            best_score: p8_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
            best_plaintext: p8_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
            mappings_tested: tested.load(Ordering::Relaxed),
            elapsed_ms: p8_start.elapsed().as_millis(),
        });
        all_candidates.extend(p8_results);
    }

    // ── Phase 9: Vigenère / Polyalphabetic ──────────────
    let p9_start = Instant::now();
    let p9_results = phase9_vigenere(&tested);
    let p9_best = p9_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P9:Vigenere",
        best_score: p9_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p9_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p9_start.elapsed().as_millis(),
    });
    all_candidates.extend(p9_results);

    // ── Phase 10: Elgar-Speak Hill Climbing ──────────────
    let p10_start = Instant::now();
    all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let top_for_elgar: Vec<[u8; 24]> = all_candidates.iter()
        .take(5)
        .map(|c| c.mapping)
        .collect();
    let p10_results = phase10_elgar_speak(&top_for_elgar, config, &tested, &start);
    let p10_best = p10_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P10:ElgarSpeak",
        best_score: p10_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p10_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p10_start.elapsed().as_millis(),
    });
    all_candidates.extend(p10_results);

    // ── Phase 11: Crib-Pinned Musical Message ────────────
    let p11_start = Instant::now();
    let p11_results = phase11_crib_pinned(config, &tested, &start);
    let p11_best = p11_results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    phase_results.push(PhaseResult {
        phase: "P11:CribPin",
        best_score: p11_best.map(|c| c.score).unwrap_or(f64::NEG_INFINITY),
        best_plaintext: p11_best.map(|c| c.plaintext.clone()).unwrap_or_default(),
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: p11_start.elapsed().as_millis(),
    });
    all_candidates.extend(p11_results);

    // ── Final Ranking ────────────────────────────────────
    all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    all_candidates.dedup_by(|a, b| a.plaintext == b.plaintext);
    all_candidates.truncate(50); // Keep top 50

    DorabellaResult {
        candidates: all_candidates,
        mappings_tested: tested.load(Ordering::Relaxed),
        elapsed_ms: start.elapsed().as_millis(),
        phase_results,
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_mapping_is_permutation() {
        let mut rng = Rng::new(42);
        let mapping = random_mapping(&mut rng);
        let mut seen = [false; 26];
        for &l in &mapping {
            assert!(l >= b'A' && l <= b'Z');
            seen[(l - b'A') as usize] = true;
        }
        // At least 20 distinct letters used (24 slots, 26 letters)
        let count = seen.iter().filter(|&&b| b).count();
        assert!(count >= 20, "Expected 20+ distinct letters, got {}", count);
    }

    #[test]
    fn test_order_crossover_valid() {
        let mut rng = Rng::new(42);
        let parent_a = random_mapping(&mut rng);
        let parent_b = random_mapping(&mut rng);
        let child = order_crossover(&parent_a, &parent_b, &mut rng);
        // All entries should be valid letters
        for &l in &child {
            assert!(l >= b'A' && l <= b'Z');
        }
        // No duplicate letters
        let mut seen = std::collections::HashSet::new();
        for &l in &child {
            assert!(seen.insert(l), "Duplicate letter {} in child", l as char);
        }
    }

    #[test]
    fn test_phase1_produces_candidate() {
        let tested = AtomicU64::new(0);
        let c = phase1_frequency(&tested);
        assert_eq!(c.plaintext.len(), 87);
        assert!(c.score.is_finite());
    }

    #[test]
    fn test_quick_attack() {
        let config = DorabellaConfig {
            hill_climb_secs: 2,
            hill_climb_restarts: 3,
            anneal_secs: 2,
            anneal_temp: 10.0,
            anneal_cooling: 0.999,
            genetic_pop: 20,
            genetic_gens: 50,
            try_musical: false,
            timeout_secs: 10,
            ..Default::default()
        };
        let result = attack(&config);
        assert!(!result.candidates.is_empty());
        assert!(result.mappings_tested > 0);
        // Best candidate may be 87 chars (full) or 80-86 chars (null-stripped)
        let len = result.candidates[0].plaintext.len();
        assert!(len >= 80 && len <= 87, "Plaintext length {} out of range", len);
    }
}
