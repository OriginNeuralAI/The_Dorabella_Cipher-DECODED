/// Dorabella Cipher — Multi-Phase Cryptanalysis Module
///
/// Attacks Edward Elgar's 1897 cipher (87 characters, 24-symbol alphabet)
/// by exploiting the structural properties of Dorabella's symbol space
/// (8 orientations x 3 arc counts = 24 symbols).
///
/// The attack pipeline:
///   Phase 1: Frequency-matched initial mapping (symbol freq -> English freq)
///   Phase 2: Parallel hill-climbing with multi-restart
///   Phase 3: Simulated annealing from best-so-far
///   Phase 4: Genetic algorithm with order crossover
///   Phase 5: Crib dragging (Victorian-era contextual fragments)
///   Phase 6: Basin clustering (orientation groups -> vowel/consonant)
///   Phase 7: Spectral refinement (IC + rank correlation bonus)
///   Phase 8: Musical hypothesis branch (circle of fifths, solfege, etc.)
///
/// Usage (reference only — not a standalone crate):
///   ```ignore
///   let result = attack(&DorabellaConfig::default());
///   println!("Best: {} (score {:.2})", result.candidates[0].plaintext, result.candidates[0].score);
///   ```

pub mod symbols;
pub mod frequency;
pub mod engine;
pub mod musical;
pub mod vigenere;

// Re-export public API
pub use engine::{DorabellaConfig, DorabellaResult, Candidate, PhaseResult, attack};
pub use symbols::{Symbol, Direction, CIPHERTEXT, CIPHER_LEN, HAUER_GLYPHS};
pub use musical::{MusicalResult, run_all_hypotheses};
pub use vigenere::{PolyAnalysis, full_poly_analysis};
