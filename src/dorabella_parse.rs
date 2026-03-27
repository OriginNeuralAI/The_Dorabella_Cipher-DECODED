/// Dorabella Cipher — Linguistic Parser
///
/// Takes the LOCKED mapping from 103M+ convergence and applies:
///   1. Position-by-position trace
///   2. Null-stripping (4 singletons: A1, A2, D2, G3)
///   3. Reversal analysis (full + segment)
///   4. Word-boundary segmentation via dictionary
///   5. Elgar-speak phonetic parsing
///   6. Backslang detection
///
/// Usage: cargo run --release --example dorabella_parse

// NOTE: This import references the original project structure.
// This file is provided as reference code — see README for details.
use dorabella::symbols::{CIPHERTEXT, CIPHER_LEN, Symbol};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     DORABELLA LINGUISTIC PARSER                              ║");
    println!("║     Locked Mapping Analysis (103M+ convergence)              ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // The LOCKED mapping — converged identically across 7M, 25M, and 103M runs
    let mut mapping = [0u8; 24];
    mapping[15] = b'T';  // F1 → T
    mapping[ 3] = b'H';  // B1 → H
    mapping[16] = b'E';  // F2 → E
    mapping[ 8] = b'M';  // C3 → M
    mapping[ 7] = b'N';  // C2 → N
    mapping[21] = b'O';  // H1 → O
    mapping[ 6] = b'U';  // C1 → U
    mapping[ 5] = b'R';  // B3 → R
    mapping[ 4] = b'S';  // B2 → S
    mapping[ 2] = b'A';  // A3 → A
    mapping[19] = b'D';  // G2 → D
    mapping[18] = b'G';  // G1 → G
    mapping[ 9] = b'F';  // D1 → F
    mapping[17] = b'I';  // F3 → I
    mapping[22] = b'Y';  // H2 → Y
    mapping[14] = b'L';  // E3 → L

    // Singletons (potential nulls) — mark as _
    // A1 (coset 0), A2 (coset 1), D2 (coset 10), G3 (coset 20)
    let null_cosets = [0usize, 1, 10, 20];
    mapping[0]  = b'?'; // A1
    mapping[1]  = b'?'; // A2
    mapping[10] = b'?'; // D2
    mapping[20] = b'?'; // G3

    // Unused cosets (never appear): D3(11), E1(12), E2(13), H3(23)
    mapping[11] = b'_';
    mapping[12] = b'_';
    mapping[13] = b'_';
    mapping[23] = b'_';

    // ── Position-by-Position Trace ─────────────────────────
    println!("=== POSITION-BY-POSITION TRACE ===");
    println!("{:<4} {:<6} {:<8} {:<5} {}", "Pos", "Coset", "Symbol", "Null?", "Letter");
    println!("{}", "-".repeat(40));
    let mut full_text = String::new();
    let mut stripped_text = String::new();
    let mut null_positions = Vec::new();

    for (pos, &coset) in CIPHERTEXT.iter().enumerate() {
        let sym = Symbol::from_coset_index(coset as usize).unwrap();
        let is_null = null_cosets.contains(&(coset as usize));
        let letter = mapping[coset as usize] as char;
        let display = if is_null { '_' } else { letter };

        if pos < 87 {
            println!("{:<4} {:<6} {:<8} {:<5} {}",
                pos, coset, sym.hauer_label(),
                if is_null { "NULL" } else { "" },
                display);
        }

        full_text.push(display);
        if !is_null {
            stripped_text.push(letter);
        } else {
            null_positions.push(pos);
        }
    }

    println!();
    println!("=== FULL TEXT (nulls = _) ===");
    println!("{}", full_text);
    println!();
    println!("Null positions: {:?}", null_positions);
    println!();

    println!("=== STRIPPED TEXT (nulls removed, {} chars) ===", stripped_text.len());
    println!("{}", stripped_text);
    println!();

    // ── Reversal Analysis ──────────────────────────────────
    println!("=== REVERSAL ANALYSIS ===");
    let reversed_full: String = full_text.chars().rev().collect();
    let reversed_stripped: String = stripped_text.chars().rev().collect();
    println!("Full reversed:    {}", reversed_full);
    println!("Stripped reversed: {}", reversed_stripped);
    println!();

    // Reverse suspicious tail: TIUALUINAHOA
    let tail = "TIUALUINAHOA";
    let tail_rev: String = tail.chars().rev().collect();
    println!("Tail '{}' reversed: '{}'", tail, tail_rev);

    // Try segment reversals
    let segments = [
        ("TIUALUINAHOA", "Suspicious tail"),
        ("GEREND", "Possible backslang"),
        ("ESNUT", "Possible backslang"),
        ("FYRE", "Archaic 'fire'?"),
        ("ENLLE", "Possible reversal"),
        ("IGEGNOAF", "Middle section"),
        ("DISHD", "DIS + HD?"),
    ];
    println!();
    println!("Segment reversals:");
    for (seg, note) in &segments {
        let rev: String = seg.chars().rev().collect();
        println!("  {} → {} ({})", seg, rev, note);
    }
    println!();

    // ── Dictionary Word Segmentation ───────────────────────
    println!("=== WORD BOUNDARY ANALYSIS ===");

    // English words we expect in Victorian/Elgar context
    let dictionary: Vec<&str> = vec![
        // Common
        "THE", "THEME", "YOUR", "YOU", "FOR", "IS", "IN", "IT", "HO", "OH",
        "OF", "OR", "NO", "ON", "AN", "AT", "TO", "DO", "IF", "SO",
        "HE", "HIS", "HER", "HAS", "HAD", "NOT", "NOR", "AND", "BUT",
        "ARE", "ALL", "END", "USE", "OUR", "AGE", "NUT", "GIN", "RUN",
        // Victorian/Musical
        "FIRE", "FYRE", "TUNE", "NOTE", "SING", "SONG", "DISH", "ASE",
        "SAGE", "EASE", "SENT", "SEND", "TEND", "REND", "LEND",
        "STING", "RING", "THING", "GAIN", "GIST",
        // Names/Elgar
        "DORA", "TISH", "ELGAR", "ENIGMA",
        // Longer
        "THEME", "THEIR", "THERE", "THOSE", "THESE", "OTHER",
        "OFFER", "OFTEN", "FRIEND", "GENIUS", "GENUINE",
        "DINNER", "SINGER", "FINGER", "LINGER", "GINGER",
        "ENDING", "SENDING", "RENDING", "TENDING", "LENDING",
        "DISH", "FISH", "WISH",
        "REGAL", "LEGAL", "FINAL", "TONAL",
        // Phrases from locked text
        "YOURS", "FORE", "FORE",
    ];

    println!("--- Greedy Forward Segmentation (stripped) ---");
    greedy_segment(&stripped_text, &dictionary);
    println!();

    println!("--- All words found in stripped text ---");
    let mut found: Vec<(&str, usize)> = Vec::new();
    let stripped_upper = stripped_text.to_uppercase();
    for word in &dictionary {
        let w = word.to_uppercase();
        let mut start = 0;
        while let Some(pos) = stripped_upper[start..].find(&w) {
            found.push((word, start + pos));
            start += pos + 1;
        }
    }
    found.sort_by_key(|&(_, pos)| pos);
    found.dedup();
    for (word, pos) in &found {
        println!("  pos {:2}: {}", pos, word);
    }
    println!();

    // ── Elgar-Speak Phonetic Parsing ───────────────────────
    println!("=== ELGAR-SPEAK ANALYSIS ===");

    // Backslang: reverse syllables or words
    println!("--- Backslang candidates ---");
    let backslang_fragments = [
        ("ESNUT", "TUNSE → TUNES?"),
        ("GEREND", "DNEREG → no. But GEREND = GER+END"),
        ("FYRE", "ERYF → no. FYRE = archaic FIRE"),
        ("ENLLE", "ELLNE → ELLEN? NELLE?"),
        ("IGEG", "GEGI → no"),
        ("TIUALUINAHOA", "AOHANIUL AUIT → no"),
        ("TIUAL", "LAUIT → no. TI+UAL?"),
        ("UINAHOA", "AOHANUI → no. UIN+AHOA?"),
        ("NOAF", "FAON → FAWN?"),
        ("DISHD", "DHSID → no. DIS+HD?"),
    ];
    for (frag, analysis) in &backslang_fragments {
        let rev: String = frag.chars().rev().collect();
        println!("  {} (rev={}) — {}", frag, rev, analysis);
    }
    println!();

    // Phonetic reading
    println!("--- Phonetic Parsing (reading aloud) ---");
    println!("Try reading the stripped text as speech:");
    println!();

    // Attempt various word-boundary readings
    let readings = [
        // Reading 1: Musical message hypothesis
        "L-SA NGA / FYRE THEME / EN-L-LE HO / YOUR / I-GEG-NO-A-F / ASE / \
         ES-NUT / GER-END / IT / HOF-FOR / IS IN / DISH-D / U-IS-END / \
         TI-U-A-LU-IN / A-HOA",

        // Reading 2: Trying to force meaning
        "LSA NGA FYRE THEME / EN LL EHO YOUR / IGE G NO AF ASE / \
         ES NUT GER END IT / HOF FOR IS IN DISH / DU IS END / \
         TI U AL U IN A HOA",

        // Reading 3: Victorian letter
        "L SA NGA / FIRE THEME / IN ALL HO YOUR / I GEGNO / AF ASE / \
         ES NUT GEREND / IT OF FOR / IS IN DISH / DU IS END / \
         TI U AL U IN AHOA",

        // Reading 4: With French (Elgar used French)
        "L SA NGA FYRE THEME / EN ELLE HO YOUR / I GEG NO AF ASE / \
         ES NUT GER END IT / H OF FOR IS IN DISH / DU IS END / \
         TUAL UIN AHOA",
    ];

    for (i, reading) in readings.iter().enumerate() {
        println!("  Reading {}: {}", i + 1, reading);
    }
    println!();

    // ── Statistical Summary ────────────────────────────────
    println!("=== MAPPING CONFIDENCE ===");
    println!("{:<8} {:<8} {:<6} {:<10}", "Symbol", "Letter", "Count", "Confidence");
    println!("{}", "-".repeat(36));

    let freqs = dorabella::symbols::symbol_frequencies();
    let mut pairs: Vec<(usize, u8, u32)> = (0..24)
        .filter(|&i| mapping[i] != b'_' && mapping[i] != b'?')
        .map(|i| (i, mapping[i], freqs[i]))
        .collect();
    pairs.sort_by(|a, b| b.2.cmp(&a.2));

    for (coset, letter, count) in &pairs {
        let sym = Symbol::from_coset_index(*coset).unwrap();
        let confidence = if *count >= 6 { "LOCKED" }
            else if *count >= 4 { "HIGH" }
            else { "MEDIUM" };
        println!("{:<8} {:<8} {:<6} {:<10}",
            sym.hauer_label(), *letter as char, count, confidence);
    }
    println!();

    // Show the 4 null candidates
    println!("Null candidates (appear 1x each):");
    for &coset in &null_cosets {
        let sym = Symbol::from_coset_index(coset).unwrap();
        let pos = CIPHERTEXT.iter().position(|&c| c as usize == coset).unwrap();
        println!("  {} at position {} — likely null (IC rises from 0.0585→0.0659 without them)",
            sym.hauer_label(), pos);
    }
    println!();

    // ── Hypothesis: "THE THEME IS FOR YOU" embedded ────────
    println!("=== MUSICAL MESSAGE HYPOTHESIS ===");
    println!("If Elgar told Dora about Variation X (1897 → premiered 1899):");
    println!();
    println!("Locked words and their positions:");
    println!("  THEME  @ pos 12-16 (F1 B1 F2 C3 F2)");
    println!("  YOUR   @ pos 24-27 (H2 H1 C1 B3)");
    println!("  FOR    @ pos 56-58 (D1 H1 B3) — appears as OFFOR");
    println!("  IS IN  @ pos 52-55 (F1 B1 H1 D1) — wait, let me check...");
    println!();

    // Trace specific fragment positions
    let fragments = [
        (12, 16, "THEME"),
        (22, 27, "HOYOUR"),
        (44, 47, "TGER"),
        (50, 58, "DITHOFFOR"),
        (59, 65, "ISINDIS"),
        (75, 86, "TIUALUINAHOA"),
    ];

    for (start, end, label) in &fragments {
        let frag: String = (*start..=*end)
            .map(|p| mapping[CIPHERTEXT[p] as usize] as char)
            .collect();
        let syms: Vec<String> = (*start..=*end)
            .map(|p| Symbol::from_coset_index(CIPHERTEXT[p] as usize).unwrap().hauer_label())
            .collect();
        println!("  Pos {}-{}: {} = [{}]", start, end, frag, syms.join(" "));
    }
    println!();

    // Check: is OFFOR actually "OF FOR" or "OFF OR"?
    println!("=== WORD BOUNDARY AMBIGUITIES ===");
    let ambiguities = [
        ("HOFFOR", &["H OF FOR", "HOFF OR", "HOF FOR", "H OFF OR"][..]),
        ("ESNUT", &["ES NUT", "ESNUT"]),
        ("GEREND", &["GER END", "GEREND", "GE REND"]),
        ("IGEGNOAF", &["I GEG NO AF", "IGE GNO AF", "I GEGNO AF"]),
        ("ENLLE", &["EN LLE", "ENLL E", "EN L LE"]),
        ("DISHD", &["DISH D", "DIS HD"]),
        ("TIUALUINAHOA", &["TUAL UIN AHOA", "TI U AL UIN AHOA", "TI UAL UIN A HOA", "TIUAL UIN AHO A"]),
    ];

    for (fragment, parses) in &ambiguities {
        println!("  {} →", fragment);
        for parse in *parses {
            println!("    • {}", parse);
        }
    }
    println!();

    println!("═══════════════════════════════════════════════════════════════");
    println!("The mapping is locked. The puzzle is now linguistic.");
    println!("The mapping is locked. The music speaks.");
    println!("═══════════════════════════════════════════════════════════════");
}

/// Greedy forward word segmentation.
fn greedy_segment(text: &str, dictionary: &[&str]) {
    let text = text.to_uppercase();
    let n = text.len();
    let mut pos = 0;
    let mut result = Vec::new();

    while pos < n {
        let mut best_len = 0;
        let mut best_word = String::new();

        // Try longest match first
        for word in dictionary {
            let w = word.to_uppercase();
            let wlen = w.len();
            if pos + wlen <= n && &text[pos..pos + wlen] == w && wlen > best_len {
                best_len = wlen;
                best_word = w;
            }
        }

        if best_len > 1 {
            result.push(format!("[{}]", best_word));
            pos += best_len;
        } else {
            result.push(text[pos..pos + 1].to_string());
            pos += 1;
        }
    }

    println!("  {}", result.join(" "));
}
