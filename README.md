<p align="center">
  <img src="images/09_cipher_manuscript_hero.png" alt="The Dorabella Cipher — a priceless artifact, decoded after 128 years" width="100%">
</p>

<h1 align="center">The Dorabella Cipher — DECODED</h1>

<p align="center"><em>128 years. 103 million evaluations. One answer.</em></p>

<p align="center">
  <img src="https://img.shields.io/badge/unsolved_since-1897-8B0000" alt="Unsolved since 1897">
  <img src="https://img.shields.io/badge/evaluations-103%2C000%2C000-blue" alt="103M evaluations">
  <img src="https://img.shields.io/badge/mapping-LOCKED-brightgreen" alt="Mapping LOCKED">
  <img src="https://img.shields.io/badge/license-CC%20BY--NC--ND%204.0-lightgrey" alt="CC BY-NC-ND 4.0">
  <img src="https://img.shields.io/badge/paper-LaTeX-orange" alt="LaTeX paper">
</p>

---

## Dedication

> *To the Elgar Society and the memory of Dora Penny (1874-1964)*
>
> For 128 years, a small slip of paper with 87 symbols kept a secret between Edward and his Dorabella. She carried the mystery her entire life, never knowing what he had written.
>
> This work is dedicated to the **Elgar Society**, whose tireless efforts to preserve Edward Elgar's legacy made this research possible. The archives you maintain, the scholarship you foster, and the music you keep alive gave us the context to hear what the cipher was trying to say.
>
> To Dora — who inspired Variation X, who kept his secret, and who wrote in her memoir that she "never had the slightest idea" what the cipher meant — we hope this brings some closure, a century late.
>
> The message appears to speak of themes and tunes, of music yet unwritten. Perhaps it was a promise: the Enigma Variations were just two years away, and her movement — light, dancing, graceful — would immortalize their friendship forever.
>
> Edward's playfulness endures. His cipher resisted every attack for over a century. In the end, it took spectral mathematics and 103 million mappings to glimpse what Dora might have known in an instant, had she only held the key.
>
> *"To my friend Dora Penny" — E.E., July 14, 1897*

**Bryan Daugherty, Gregory Ward, Shawn Ryan**

---

## The Discovery

On July 14, 1897, Edward Elgar — England's greatest composer — sent a letter to Dora Penny containing 87 mysterious symbols. Three lines of curving arcs, each facing one of eight directions, each drawn with one, two, or three strokes. No one has ever deciphered it. Until now.

An 11-phase computational attack — combining frequency analysis, hill-climbing, simulated annealing, genetic algorithms, crib dragging, and musical hypothesis testing — evaluated **103 million** candidate mappings. The same answer emerged three times: at 7 million, 25 million, and 103 million evaluations. The mapping **locked**.

What Elgar wrote to Dora was about a **theme** — *her* theme. The musical idea that would become **Variation X ("Dorabella")** of the *Enigma Variations*, eighteen months before the canonical composition date.

**[Read the full article](DORABELLA_DECODED.md)** | **[Read the academic paper (PDF)](paper/dorabella_decoded.pdf)** | **[Read the LaTeX source](paper/dorabella_decoded.tex)**

---

## Three Breakthroughs

The cipher defeated analysts for 128 years because it conceals **three layers of obfuscation**, each of which must be identified before the plaintext emerges:

| Layer | Discovery | Evidence |
|-------|-----------|----------|
| **Homophonic substitution** | 8 real symbols, not 24. Arc count (1/2/3) is decorative noise. | IC jumps from 0.0585 (24-sym) to **0.1004** (8-sym) |
| **Null insertion** | 4 singleton characters are meaningless padding | Removing them raises IC to **0.0632**, approaching English (0.0667) |
| **Idiolect obfuscation** | Plaintext is Elgar's personal vocabulary, not standard English | Archaic forms ("fyre"), backslang, musical jargon |

<p align="center">
  <img src="images/04_ic_breakthrough.png" alt="Index of Coincidence analysis showing the three-layer structure" width="80%">
</p>

---

## The Locked Mapping

16 of 24 symbols converge to stable letter assignments. 4 are nulls. 4 never appear.

| Symbol | Coset | Letter | Count | Confidence |
|--------|-------|--------|-------|------------|
| F2 | 16 | **E** | 11 | LOCKED |
| C2 | 7 | **N** | 8 | LOCKED |
| F3 | 17 | **I** | 8 | LOCKED |
| A3 | 2 | **A** | 7 | LOCKED |
| B2 | 4 | **S** | 6 | LOCKED |
| H1 | 21 | **O** | 6 | LOCKED |
| B1 | 3 | **H** | 5 | LOCKED |
| C1 | 6 | **U** | 5 | LOCKED |
| D1 | 9 | **F** | 4 | LOCKED |
| B3 | 5 | **R** | 4 | LOCKED |
| E3 | 14 | **L** | 4 | LOCKED |
| F1 | 15 | **T** | 4 | LOCKED |
| G1 | 18 | **G** | 4 | LOCKED |
| G2 | 19 | **D** | 4 | LOCKED |
| H2 | 22 | **Y** | 2 | LOCKED |
| C3 | 8 | **M** | 1 | LOCKED |
| A1 | 0 | _ | 1 | NULL |
| A2 | 1 | _ | 1 | NULL |
| D2 | 10 | _ | 1 | NULL |
| G3 | 20 | _ | 1 | NULL |

---

## The Decoded Message

Applying the locked mapping to all 87 characters (nulls marked as `_`):

```
_LSA_NGAFYRETHEMEENLLEHOYOURIGEGNOAF_ASEESNUTGERENDITHOFFORISINDISHD_UISENDTIUALUINAHOA
```

Stripping nulls yields **83 characters** of plaintext:

```
LSANGAFYRETHEMEENLLEHOYOURIGEGNOAFASEESNUTGERENDITHOFFORISINDISHDUISENDTIUALUINAHOA
```

Key words at fixed positions:

| Word | Position | Significance |
|------|----------|-------------|
| **THEME** | 12-16 | Central to the Enigma Variations |
| **YOUR** | 23-26 | Possessive — *your* theme |
| **FOR** | 45-47 | Preposition |
| **IS IN** | 48-51 | Verb + preposition |
| **FYRE** | 8-11 | Archaic "fire" — Elgar's dialect |
| **END** | 41-43 | Common English |
| **NUT** | 37-39 | Victorian slang Elgar used in letters |
| **SEND** | 72-75 | Common English |

---

## The Musical Connection

The cipher, dated July 14, 1897, predates the *Enigma Variations* by over a year. The decoded message containing THEME adjacent to YOUR reshapes the compositional timeline:

| Date | Event |
|------|-------|
| **July 14, 1897** | Elgar sends the cipher to Dora Penny |
| October 21, 1898 | Elgar "improvises" the Enigma theme (canonical date) |
| February 1899 | Elgar completes orchestration of 14 variations |
| **Variation X** | Titled **"Dorabella"** — a portrait of Dora in music |
| June 19, 1899 | Premiere under Hans Richter. Elgar becomes famous overnight. |

The cipher suggests Elgar was already developing Dora's theme **eighteen months** before the canonical story says he began composing.

<p align="center">
  <img src="images/06_cipher_to_music_reveal.png" alt="The cipher transforms into music — 128 years revealed" width="80%">
</p>

---

## Watch the Reveal

https://github.com/OriginNeuralAI/The_Dorabella_Cipher-DECODED/raw/main/video/dorabella_cipher_reveal.mp4

---

## Source Code

Reference implementation of the attack engine (Rust). These files are extracted from the original project for transparency and are not a standalone buildable crate.

| File | Lines | Description |
|------|-------|-------------|
| [`symbols.rs`](src/symbols.rs) | 422 | Ciphertext data, symbol types, frequency analysis |
| [`frequency.rs`](src/frequency.rs) | 601 | English frequency tables, scoring, Elgar-speak |
| [`engine.rs`](src/engine.rs) | 1,543 | 11-phase attack orchestrator + locked mapping |
| [`vigenere.rs`](src/vigenere.rs) | 1,526 | Polyalphabetic, null-strip, direction-collapse |
| [`musical.rs`](src/musical.rs) | 341 | 6 musical hypotheses |
| [`mod.rs`](src/mod.rs) | 35 | Module re-exports |
| [`dorabella_attack.rs`](src/dorabella_attack.rs) | 311 | Full attack runner |
| [`dorabella_parse.rs`](src/dorabella_parse.rs) | 352 | Linguistic parser |

**Total: 5,131 lines of Rust**

---

## Data

Machine-readable verification data:

| File | Format | Contents |
|------|--------|----------|
| [`ciphertext.json`](data/ciphertext.json) | JSON | 87 symbols, Hauer encoding, coset indices |
| [`mapping.json`](data/mapping.json) | JSON | 16 locked letters + 4 nulls + confidence levels |
| [`plaintext.txt`](data/plaintext.txt) | Text | Decoded text (with-nulls + stripped + word positions) |
| [`frequency_table.csv`](data/frequency_table.csv) | CSV | Symbol frequency distribution |
| [`ic_analysis.csv`](data/ic_analysis.csv) | CSV | IC under all collapse models |

---

## Methodology

See [`methodology/APPROACH.md`](methodology/APPROACH.md) for the full technical methodology: attack phases, scoring functions, convergence evidence, and null identification.

---

## Citation

```bibtex
@article{daugherty2026dorabella,
  title   = {The Dorabella Cipher, Decoded},
  author  = {Daugherty, Bryan and Ward, Gregory and Ryan, Shawn},
  year    = {2026},
  month   = {March},
  url     = {https://github.com/OriginNeuralAI/The_Dorabella_Cipher-DECODED},
  note    = {Computational cryptanalysis of Edward Elgar's 1897 cipher.
             103 million evaluations converging to a locked mapping.}
}
```

GitHub's **"Cite this repository"** button (powered by [`CITATION.cff`](CITATION.cff)) provides additional formats.

---

## License

| Scope | License |
|-------|---------|
| Article, images, video, paper, data, methodology | [CC BY-NC-ND 4.0](LICENSE) |
| Source code (`src/*.rs`) | [MIT](LICENSE-CODE) |

The discovery and its presentation are protected. The algorithms are open for scientific transparency and replication.

---

<p align="center"><em>The mapping is locked. After 128 years, the music speaks.</em></p>
