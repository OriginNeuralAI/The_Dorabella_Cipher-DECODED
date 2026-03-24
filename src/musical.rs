/// Dorabella Cipher — Musical Hypothesis Branch
///
/// Elgar was one of the foremost composers of his era. The semicircular
/// symbols resemble musical notation (arcs, slurs, dynamics markings).
/// This module tests hypotheses that the cipher encodes musical content
/// rather than (or in addition to) English text.
///
/// Hypotheses tested:
///   H1: Orientation → pitch class (circle of fifths mapping)
///   H2: Arc count → duration (1=quarter, 2=half, 3=whole)
///   H3: Orientation → note letter (A-G cycling), arcs → octave/accidental
///   H4: Solfege mapping (Do-Re-Mi-Fa-Sol-La-Ti)
///   H5: Staff position mapping (line/space on treble clef)
///   H6: Enigma Variations theme reference

use std::sync::atomic::{AtomicU64, Ordering};
use super::symbols::{CIPHERTEXT, CIPHER_LEN, Symbol};
use super::engine::Candidate;
use super::frequency;

// ═══════════════════════════════════════════════════════════════
// PITCH CLASS MAPPINGS
// ═══════════════════════════════════════════════════════════════

/// Circle of fifths: C, G, D, A, E, B, F#, C#
/// Map 8 orientations (A-H) to pitch classes via circle of fifths.
const CIRCLE_OF_FIFTHS: [&str; 8] = ["C", "G", "D", "A", "E", "B", "F#", "C#"];

/// Chromatic scale (12 pitch classes).
const CHROMATIC: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Note letters (natural notes only).
const NOTE_LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];

/// Solfege syllables.
const SOLFEGE: [&str; 7] = ["Do", "Re", "Mi", "Fa", "Sol", "La", "Ti"];

/// Duration names.
const DURATIONS: [&str; 3] = ["quarter", "half", "whole"];

// ═══════════════════════════════════════════════════════════════
// MUSICAL DECRYPTION FUNCTIONS
// ═══════════════════════════════════════════════════════════════

/// H1: Map orientation to circle-of-fifths pitch, arcs to duration.
fn h1_circle_of_fifths() -> String {
    CIPHERTEXT.iter().map(|&s| {
        let sym = Symbol::from_coset_index(s as usize).unwrap();
        let pitch = CIRCLE_OF_FIFTHS[sym.direction as usize];
        format!("{}{}", pitch, sym.arcs)
    }).collect::<Vec<_>>().join(" ")
}

/// H2: Map orientation to note letter (A-G, wrapping), arcs to octave.
fn h2_note_letter_octave() -> String {
    CIPHERTEXT.iter().map(|&s| {
        let sym = Symbol::from_coset_index(s as usize).unwrap();
        let note = NOTE_LETTERS[sym.direction as usize % 7];
        let octave = sym.arcs + 3; // octave 4, 5, 6
        format!("{}{}", note, octave)
    }).collect::<Vec<_>>().join(" ")
}

/// H3: Direct alphabetic: orientation A-H maps to note A-G (+H=A'),
/// arcs indicate natural(1)/sharp(2)/flat(3).
fn h3_alphabetic_accidental() -> String {
    CIPHERTEXT.iter().map(|&s| {
        let sym = Symbol::from_coset_index(s as usize).unwrap();
        let base_note = NOTE_LETTERS[sym.direction as usize % 7];
        let accidental = match sym.arcs {
            1 => "",   // natural
            2 => "#",  // sharp
            3 => "b",  // flat
            _ => "?",
        };
        format!("{}{}", base_note, accidental)
    }).collect::<Vec<_>>().join(" ")
}

/// H4: Solfege mapping (8 orientations → 7 solfege + rest).
fn h4_solfege() -> String {
    CIPHERTEXT.iter().map(|&s| {
        let sym = Symbol::from_coset_index(s as usize).unwrap();
        let dir = sym.direction as usize;
        if dir < 7 {
            format!("{}({})", SOLFEGE[dir], DURATIONS[(sym.arcs - 1) as usize])
        } else {
            format!("rest({})", DURATIONS[(sym.arcs - 1) as usize])
        }
    }).collect::<Vec<_>>().join(" ")
}

/// H5: Staff position — treat the 24 symbols as positions on a
/// grand staff (treble + bass clef, ~24 positions covers about 2 octaves).
fn h5_staff_position() -> String {
    // Map coset 0-23 to MIDI-like note numbers centered on middle C (60)
    let base_midi = 48; // C3
    CIPHERTEXT.iter().map(|&s| {
        let midi = base_midi + s as u32;
        let note = CHROMATIC[(midi % 12) as usize];
        let octave = midi / 12;
        format!("{}{}", note, octave)
    }).collect::<Vec<_>>().join(" ")
}

/// H6: Check if the decoded pitches contain the Enigma Variations theme.
/// The "Enigma" theme starts: G-G-Ab-F-Eb (or transpositions).
fn h6_enigma_theme_test() -> (bool, String) {
    // The Enigma theme intervals (in semitones from first note):
    // G=0, G=0, Ab=+1, F=-2, Eb=-4 → intervals: [0, 1, -3, -2]
    let enigma_intervals: [i32; 4] = [0, 1, -3, -2];

    // Convert ciphertext to pitch classes (using H1 circle-of-fifths as one mapping)
    let pitches: Vec<i32> = CIPHERTEXT.iter().map(|&s| {
        let sym = Symbol::from_coset_index(s as usize).unwrap();
        // Use semitone value: circle of fifths position × 7 mod 12
        ((sym.direction as i32) * 7) % 12
    }).collect();

    // Search for Enigma theme in all transpositions
    for start in 0..pitches.len().saturating_sub(4) {
        let intervals: Vec<i32> = (1..5).map(|i| {
            if start + i < pitches.len() {
                (pitches[start + i] - pitches[start]) % 12
            } else {
                99 // impossible
            }
        }).collect();

        // Check all 12 transpositions
        if intervals.len() >= 4 && intervals[..4] == enigma_intervals {
            return (true, format!("Enigma theme found at position {}", start));
        }
    }

    (false, "No Enigma theme match found".to_string())
}

// ═══════════════════════════════════════════════════════════════
// HYBRID: MUSICAL NOTE NAMES AS TEXT
// ═══════════════════════════════════════════════════════════════

/// If the message is about music, the *text* might contain note names.
/// Map symbols to letters that spell musical terms.
/// This tests mappings where common musical words appear in the plaintext.
fn musical_word_mappings(tested: &AtomicU64) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let freqs = super::symbols::symbol_frequencies();

    // Musical words that might appear as text in a letter about music
    let musical_cribs = [
        "ENIGMA", "THEME", "ADAGIO", "ALLEGRO", "ANDANTE",
        "FORTE", "PIANO", "TEMPO", "SCHERZO", "CANTABILE",
        "FUGUE", "SONATA", "LARGHETTO", "PRESTO",
        "DOLCE", "VIVACE", "GRACE", "CADENCE",
    ];

    for crib in &musical_cribs {
        let crib_bytes: Vec<u8> = crib.bytes().collect();
        if crib_bytes.len() > CIPHER_LEN { continue; }

        // Try at each position
        for pos in 0..=(CIPHER_LEN - crib_bytes.len()) {
            let mut partial = [0u8; 24];
            let mut used_letters = [false; 26];
            let mut ok = true;

            for (i, &letter) in crib_bytes.iter().enumerate() {
                let sym = CIPHERTEXT[pos + i] as usize;
                let li = (letter - b'A') as usize;
                if partial[sym] == 0 {
                    if used_letters[li] { ok = false; break; }
                    partial[sym] = letter;
                    used_letters[li] = true;
                } else if partial[sym] != letter {
                    ok = false;
                    break;
                }
            }
            if !ok { continue; }

            // Fill remaining with frequency match
            let mut mapping = partial;
            let mut remaining: Vec<u8> = (b'A'..=b'Z')
                .filter(|l| !used_letters[(*l - b'A') as usize])
                .collect();
            remaining.sort_by(|a, b| {
                frequency::ENGLISH_FREQ[(*b - b'A') as usize]
                    .partial_cmp(&frequency::ENGLISH_FREQ[(*a - b'A') as usize])
                    .unwrap()
            });
            let mut unset: Vec<usize> = (0..24).filter(|&i| mapping[i] == 0).collect();
            unset.sort_by(|&a, &b| freqs[b].cmp(&freqs[a]));
            for (idx, &sym) in unset.iter().enumerate() {
                mapping[sym] = if idx < remaining.len() { remaining[idx] } else { b'X' };
            }

            tested.fetch_add(1, Ordering::Relaxed);
            let plaintext = super::symbols::decrypt_with_mapping(&mapping);
            let score = frequency::ensemble_score(&plaintext)
                + frequency::impossible_pattern_penalty(&plaintext);

            candidates.push(Candidate { mapping, plaintext, score, phase: "P8:Musical" });
        }
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates.truncate(50);
    candidates
}

// ═══════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════

/// Musical hypothesis result with decoded representation.
#[derive(Debug, Clone)]
pub struct MusicalResult {
    pub hypothesis: &'static str,
    pub decoded: String,
    pub description: String,
}

/// Run all musical hypotheses and return results.
pub fn run_all_hypotheses() -> Vec<MusicalResult> {
    let mut results = Vec::new();

    results.push(MusicalResult {
        hypothesis: "H1:CircleOfFifths",
        decoded: h1_circle_of_fifths(),
        description: "Orientation → circle-of-fifths pitch, arcs → duration".into(),
    });

    results.push(MusicalResult {
        hypothesis: "H2:NoteLetterOctave",
        decoded: h2_note_letter_octave(),
        description: "Orientation → note letter (A-G), arcs → octave (4-6)".into(),
    });

    results.push(MusicalResult {
        hypothesis: "H3:AlphabeticAccidental",
        decoded: h3_alphabetic_accidental(),
        description: "Orientation → note, arcs → natural/sharp/flat".into(),
    });

    results.push(MusicalResult {
        hypothesis: "H4:Solfege",
        decoded: h4_solfege(),
        description: "Orientation → Do-Re-Mi-Fa-Sol-La-Ti-rest".into(),
    });

    results.push(MusicalResult {
        hypothesis: "H5:StaffPosition",
        decoded: h5_staff_position(),
        description: "Coset index → staff position (grand staff)".into(),
    });

    let (found, detail) = h6_enigma_theme_test();
    results.push(MusicalResult {
        hypothesis: "H6:EnigmaTheme",
        decoded: detail.clone(),
        description: format!("Search for Enigma Variations theme motif (found={})", found),
    });

    results
}

/// Test musical hypotheses that produce text-like output.
/// Returns Candidates that can be compared with text-based phases.
pub fn test_musical_hypotheses(tested: &AtomicU64) -> Vec<Candidate> {
    musical_word_mappings(tested)
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h1_circle_of_fifths() {
        let result = h1_circle_of_fifths();
        assert!(!result.is_empty());
        // Should contain pitch names
        assert!(result.contains('C') || result.contains('G') || result.contains('D'));
    }

    #[test]
    fn test_h2_note_letter_octave() {
        let result = h2_note_letter_octave();
        assert!(!result.is_empty());
        // Should contain octave numbers
        assert!(result.contains('4') || result.contains('5') || result.contains('6'));
    }

    #[test]
    fn test_h3_accidentals() {
        let result = h3_alphabetic_accidental();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_h4_solfege() {
        let result = h4_solfege();
        assert!(result.contains("Do") || result.contains("Re") || result.contains("Mi"));
    }

    #[test]
    fn test_h5_staff_position() {
        let result = h5_staff_position();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_h6_enigma_theme() {
        let (_, detail) = h6_enigma_theme_test();
        assert!(!detail.is_empty());
    }

    #[test]
    fn test_all_hypotheses() {
        let results = run_all_hypotheses();
        assert_eq!(results.len(), 6);
        for r in &results {
            assert!(!r.decoded.is_empty());
            assert!(!r.description.is_empty());
        }
    }

    #[test]
    fn test_musical_word_mappings() {
        let tested = AtomicU64::new(0);
        let results = test_musical_hypotheses(&tested);
        // Should produce some candidates (musical words placed at various positions)
        assert!(!results.is_empty() || tested.load(Ordering::Relaxed) > 0);
    }
}
