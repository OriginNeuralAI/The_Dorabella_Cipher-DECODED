/// Dorabella Cipher Attack — 2026
///
/// Runs the full 11-phase cryptanalysis against Elgar's 1897 cipher,
/// exploiting the 24-symbol structural properties of Dorabella's alphabet.
///
/// Phases 1-8: MASC attack (frequency, hill-climb, SA, genetic, crib, basin, spectral, musical)
/// Phase 9:    Polyalphabetic / Vigenère (Kasiski, IC-period, musical keys, transposition, nulls)
/// Phase 10:   Elgar-Speak hill-climbing (backslang, abbreviations, Victorian patterns)
/// Phase 11:   Crib-Pinned Musical Message (frozen core + musical-message cribs + hill-climb)
///
/// Usage:
///   cargo run --release --example dorabella_attack
///   cargo run --release --example dorabella_attack -- --quick
///   cargo run --release --example dorabella_attack -- --deep
///   cargo run --release --example dorabella_attack -- --ultra

// NOTE: These imports reference the original project structure.
// This file is provided as reference code — see README for details.
use dorabella::{self, DorabellaConfig, Symbol, CIPHER_LEN};
use dorabella::symbols;
use dorabella::musical;
use dorabella::vigenere;
use colored::Colorize;
use comfy_table::{Table, ContentArrangement};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let quick = args.iter().any(|a| a == "--quick");
    let deep = args.iter().any(|a| a == "--deep");
    let ultra = args.iter().any(|a| a == "--ultra");

    println!("{}", "╔═══════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║       DORABELLA CIPHER ATTACK ENGINE                         ║".bright_cyan());
    println!("{}", "║       Multi-Phase Cryptanalysis × Elgar 1897                ║".bright_cyan());
    println!("{}", "╚═══════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();

    // ── Cipher Analysis ──────────────────────────────────
    println!("{}", "=== CIPHER ANALYSIS ===".bright_yellow().bold());
    println!("Ciphertext length:    {} symbols", CIPHER_LEN);
    println!("Unique symbols used:  {} of 24", symbols::unique_symbol_count());
    println!("Unused symbols:       {:?}", symbols::unused_symbols().iter()
        .map(|&i| Symbol::from_coset_index(i).unwrap().hauer_label())
        .collect::<Vec<_>>());
    println!("Index of Coincidence: {:.4}", symbols::index_of_coincidence());
    println!("  (English mono ≈ 0.0667, random/24 ≈ 0.0417, random/26 ≈ 0.0385)");
    println!();

    // Frequency table
    println!("{}", "--- Symbol Frequencies ---".bright_white());
    let sorted = symbols::sorted_frequencies();
    let mut freq_table = Table::new();
    freq_table.set_content_arrangement(ContentArrangement::Dynamic);
    freq_table.set_header(vec!["Symbol", "Coset", "Count", "Proportion", "Bar"]);
    for (idx, count) in &sorted {
        if *count == 0 { continue; }
        let sym = Symbol::from_coset_index(*idx).unwrap();
        let prop = *count as f64 / CIPHER_LEN as f64;
        let bar = "#".repeat((*count as usize).min(30));
        freq_table.add_row(vec![
            sym.hauer_label(),
            format!("{:2}", idx),
            format!("{:2}", count),
            format!("{:.3}", prop),
            bar,
        ]);
    }
    println!("{}", freq_table);
    println!();

    // Top bigrams
    println!("{}", "--- Top 10 Bigrams ---".bright_white());
    let bigrams = symbols::bigram_frequencies();
    for (i, ((a, b), count)) in bigrams.iter().take(10).enumerate() {
        let sa = Symbol::from_coset_index(*a as usize).unwrap();
        let sb = Symbol::from_coset_index(*b as usize).unwrap();
        println!("  {:2}. {} {} → count {}", i + 1, sa, sb, count);
    }
    println!();

    // Top trigrams
    println!("{}", "--- Top 10 Trigrams ---".bright_white());
    let trigrams = symbols::trigram_frequencies();
    for (i, ((a, b, c), count)) in trigrams.iter().take(10).enumerate() {
        let sa = Symbol::from_coset_index(*a as usize).unwrap();
        let sb = Symbol::from_coset_index(*b as usize).unwrap();
        let sc = Symbol::from_coset_index(*c as usize).unwrap();
        println!("  {:2}. {} {} {} → count {}", i + 1, sa, sb, sc, count);
    }
    println!();

    // ── Musical Hypotheses ───────────────────────────────
    println!("{}", "=== MUSICAL HYPOTHESES ===".bright_yellow().bold());
    let musical_results = musical::run_all_hypotheses();
    for result in &musical_results {
        println!("{}: {}", result.hypothesis.bright_green(), result.description);
        // Show first 80 chars of decoded
        let preview: String = result.decoded.chars().take(80).collect();
        println!("  → {}...", preview);
        println!();
    }

    // ── Polyalphabetic / Non-MASC Analysis ────────────────
    println!("{}", "=== POLYALPHABETIC ANALYSIS (Wase 2023: 'unlikely MASC') ===".bright_yellow().bold());

    // Kasiski examination
    let kasiski = vigenere::kasiski_examination();
    println!("{}", "--- Kasiski Examination (top 10 repeated n-grams) ---".bright_white());
    for (i, k) in kasiski.iter().take(10).enumerate() {
        let ngram_labels: Vec<String> = k.ngram.iter()
            .map(|&s| Symbol::from_coset_index(s as usize).unwrap().hauer_label())
            .collect();
        println!("  {:2}. [{}] at {:?} spacings={:?} GCD={}",
            i + 1, ngram_labels.join(" "), k.positions, k.spacings, k.gcd);
    }
    println!();

    // IC per period
    let period_ics = vigenere::ic_per_period(12);
    println!("{}", "--- IC by Period (closer to 0.0667 = more likely Vigenere) ---".bright_white());
    let mut ic_table = Table::new();
    ic_table.set_content_arrangement(ContentArrangement::Dynamic);
    ic_table.set_header(vec!["Period", "Avg IC", "Columns", "Delta from English"]);
    for pic in &period_ics {
        let delta = (pic.avg_ic - 0.0667).abs();
        ic_table.add_row(vec![
            format!("{}", pic.period),
            format!("{:.4}", pic.avg_ic),
            format!("{}", pic.columns),
            format!("{:.4}", delta),
        ]);
    }
    println!("{}", ic_table);
    println!();

    // Homophonic hypothesis
    let homo = vigenere::homophonic_hypothesis();
    println!("{}", "--- Homophonic Hypothesis ---".bright_white());
    for h in &homo {
        let expected_ic = 1.0 / h.effective_alphabet as f64;
        println!("  {}: alphabet={}, IC={:.4} (random={:.4}, English=0.0667)",
            h.name, h.effective_alphabet, h.ic, expected_ic);
    }
    println!();

    // Null symbol candidates
    let nulls = vigenere::null_symbol_candidates();
    println!("{}", "--- Null Symbol Hypothesis ---".bright_white());
    for n in &nulls {
        let removed_labels: Vec<String> = n.removed.iter()
            .map(|&i| Symbol::from_coset_index(i).unwrap().hauer_label())
            .collect();
        println!("  Remove [{}]: {} chars remain, IC={:.4}",
            removed_labels.join(", "), n.remaining_len, n.ic);
    }
    println!();

    // ── Direction-Only Analysis ────────────────────────────
    println!("{}", "=== DIRECTION-ONLY ANALYSIS (arc-count = noise) ===".bright_yellow().bold());
    let dir_freq = vigenere::direction_frequencies();
    let dir_ic = vigenere::direction_ic();
    let dir_labels = ["A", "B", "C", "D", "E", "F", "G", "H"];
    println!("Direction IC: {:.4} (random/8 = 0.1250, English/8 equiv ≈ 0.1500)", dir_ic);
    println!();
    println!("{}", "--- Direction Frequencies ---".bright_white());
    let mut dir_table = Table::new();
    dir_table.set_content_arrangement(ContentArrangement::Dynamic);
    dir_table.set_header(vec!["Dir", "Count", "Prop", "Bar"]);
    let mut dir_sorted: Vec<(usize, u32)> = dir_freq.iter().enumerate()
        .map(|(i, &f)| (i, f))
        .collect();
    dir_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for (i, count) in &dir_sorted {
        let prop = *count as f64 / CIPHER_LEN as f64;
        let bar = "#".repeat((*count as usize).min(30));
        dir_table.add_row(vec![
            dir_labels[*i].to_string(),
            format!("{:2}", count),
            format!("{:.3}", prop),
            bar,
        ]);
    }
    println!("{}", dir_table);
    println!();

    // ── Attack Configuration ─────────────────────────────
    let config = if ultra {
        println!("{}", "=== ULTRA ATTACK MODE ===".bright_red().bold());
        DorabellaConfig {
            hill_climb_secs: 300,
            hill_climb_restarts: 500,
            anneal_secs: 300,
            anneal_temp: 40.0,
            anneal_cooling: 0.999999,
            genetic_pop: 2000,
            genetic_gens: 50_000,
            try_musical: true,
            timeout_secs: 1200,
            ..Default::default()
        }
    } else if quick {
        println!("{}", "=== QUICK ATTACK MODE ===".bright_yellow().bold());
        DorabellaConfig {
            hill_climb_secs: 5,
            hill_climb_restarts: 10,
            anneal_secs: 10,
            anneal_temp: 15.0,
            anneal_cooling: 0.999,
            genetic_pop: 100,
            genetic_gens: 500,
            try_musical: true,
            timeout_secs: 30,
            ..Default::default()
        }
    } else if deep {
        println!("{}", "=== DEEP ATTACK MODE ===".bright_yellow().bold());
        DorabellaConfig {
            hill_climb_secs: 120,
            hill_climb_restarts: 200,
            anneal_secs: 180,
            anneal_temp: 30.0,
            anneal_cooling: 0.99999,
            genetic_pop: 1000,
            genetic_gens: 20_000,
            try_musical: true,
            timeout_secs: 600,
            ..Default::default()
        }
    } else {
        println!("{}", "=== STANDARD ATTACK MODE ===".bright_yellow().bold());
        DorabellaConfig::default()
    };

    println!("Hill-climb: {} restarts × {}s", config.hill_climb_restarts, config.hill_climb_secs);
    println!("Annealing:  T₀={}, cooling={}, {}s", config.anneal_temp, config.anneal_cooling, config.anneal_secs);
    println!("Genetic:    pop={}, gens={}", config.genetic_pop, config.genetic_gens);
    println!("Timeout:    {}s", config.timeout_secs);
    println!();

    // ── Run Attack ───────────────────────────────────────
    println!("{}", ">>> LAUNCHING 11-PHASE ATTACK <<<".bright_red().bold());
    println!();

    let result = dorabella::attack(&config);

    // ── Phase Results ────────────────────────────────────
    println!();
    println!("{}", "=== PHASE RESULTS ===".bright_yellow().bold());
    let mut phase_table = Table::new();
    phase_table.set_content_arrangement(ContentArrangement::Dynamic);
    phase_table.set_header(vec!["Phase", "Best Score", "Time (ms)", "Mappings", "Best Preview"]);
    for pr in &result.phase_results {
        let preview: String = pr.best_plaintext.chars().take(40).collect();
        phase_table.add_row(vec![
            pr.phase.to_string(),
            format!("{:.2}", pr.best_score),
            format!("{}", pr.elapsed_ms),
            format!("{}", pr.mappings_tested),
            preview,
        ]);
    }
    println!("{}", phase_table);
    println!();

    // ── Top Candidates ───────────────────────────────────
    println!("{}", "=== TOP 20 CANDIDATES ===".bright_yellow().bold());
    let show = result.candidates.len().min(20);
    for (rank, candidate) in result.candidates.iter().take(show).enumerate() {
        println!("{}",
            format!("#{:2} [{}] score={:.2}", rank + 1, candidate.phase, candidate.score)
                .color(if rank == 0 { "bright_green" } else if rank < 5 { "green" } else { "white" })
        );
        // Show plaintext with word-length groupings
        let pt = &candidate.plaintext;
        println!("    {}", pt);

        // Show mapping
        if rank < 3 {
            let mut mapping_str = String::new();
            for (i, &letter) in candidate.mapping.iter().enumerate() {
                let sym = Symbol::from_coset_index(i).unwrap();
                mapping_str.push_str(&format!("{}→{} ", sym.hauer_label(), letter as char));
            }
            println!("    Mapping: {}", mapping_str.dimmed());
        }
        println!();
    }

    // ── Summary ──────────────────────────────────────────
    println!("{}", "=== SUMMARY ===".bright_yellow().bold());
    println!("Total mappings tested: {}", result.mappings_tested);
    println!("Wall-clock time:       {:.1}s", result.elapsed_ms as f64 / 1000.0);
    println!("Phases completed:      {}", result.phase_results.len());

    if let Some(best) = result.candidates.first() {
        println!();
        println!("{}", "BEST DECRYPTION:".bright_green().bold());
        println!("  Score:     {:.2}", best.score);
        println!("  Phase:     {}", best.phase);
        println!("  Plaintext: {}", best.plaintext.bright_white().bold());

        // IC of best plaintext
        let ic = dorabella::frequency::text_ic(&best.plaintext);
        let rank_corr = dorabella::frequency::frequency_rank_correlation(&best.plaintext);
        println!("  IC:        {:.4} (English ≈ 0.0667)", ic);
        println!("  Rank ρ:    {:.4} (perfect = 1.0)", rank_corr);
    }

    println!();
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "The mapping is locked. The cipher speaks.".bright_cyan().italic());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
}
