//! Pure metrics comparing transcription × reference.
//!
//! Three axes, measured separately on purpose (see doc 08 §7):
//! - **WER** — content: did the right words come out?
//! - **PER** — punctuation (Picovoice style, marks `{. , ! ?}`): did the
//!   punctuation come out in the right place?
//! - **cap_mismatches** — capitalization: between equal words, does the case match?
//!
//! No function here does I/O — everything unit-tested with pt-BR examples.

/// Punctuation marks that PER evaluates (Picovoice style).
const PER_MARKS: [char; 4] = ['.', ',', '!', '?'];

/// Punctuation removed during normalization. Includes typographic quotes and dashes;
/// the hyphen is also here — see the note in [`normalize_words`].
fn is_punctuation(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '…'
            | '"'
            | '\''
            | '('
            | ')'
            | '—'
            | '–'
            | '-'
            | '\u{201C}' // “
            | '\u{201D}' // ”
            | '\u{2018}' // ‘
            | '\u{2019}' // ’
            | '«'
            | '»'
    )
}

/// A word from the original text with everything the metrics need from it.
struct Token {
    /// Original form (no punctuation, case preserved) — for the cap check.
    original: String,
    /// Normalized form (lowercase) — basis of the WER/PER alignment.
    normalized: String,
    /// Punctuation mark immediately after the word, if it is one of
    /// [`PER_MARKS`]. The first mark found before the next word
    /// wins ("sério?!" → '?').
    mark: Option<char>,
}

/// Tokenizes while preserving the word → adjacent punctuation relationship.
///
/// Known limitations (sufficient for the golden set):
/// - the mark is assigned to the previous word even with a space in between
///   ("palavra ." counts as "palavra.");
/// - `…`, `;` and `:` are not PER marks — they are simply removed;
/// - a mark before the first word is ignored.
fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_whitespace() || is_punctuation(c) {
            if !current.is_empty() {
                tokens.push(Token {
                    normalized: current.to_lowercase(),
                    original: std::mem::take(&mut current),
                    mark: None,
                });
            }
            if PER_MARKS.contains(&c) {
                if let Some(previous) = tokens.last_mut() {
                    if previous.mark.is_none() {
                        previous.mark = Some(c);
                    }
                }
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(Token {
            normalized: current.to_lowercase(),
            original: current,
            mark: None,
        });
    }
    tokens
}

/// Normalizes a text for content comparison: lowercase, punctuation
/// removed (accents and diacritics PRESERVED — pt-BR!), spaces collapsed.
///
/// **Hyphen becomes a separator**: "guarda-chuva" → `["guarda", "chuva"]`. A
/// deliberate choice — providers sometimes hyphenate, sometimes not, and treating
/// it as a separator makes the two spellings equivalent in WER.
#[allow(dead_code)] // part of the module API; exercised in the unit tests.
pub fn normalize_words(text: &str) -> Vec<String> {
    tokenize(text).into_iter().map(|t| t.normalized).collect()
}

/// Operation of the Levenshtein alignment between reference and hypothesis.
/// Indices point to the tokens on each side.
enum Op {
    /// Equal normalized words.
    Equal(usize, usize),
    /// Substitution.
    Substitution(usize, usize),
    /// Reference word missing in the hypothesis (deletion).
    Deletion(usize),
    /// Extra word in the hypothesis (insertion).
    Insertion(usize),
}

/// Levenshtein distance over words + alignment backtrace.
/// Ties prefer the diagonal (match/substitution) — a denser alignment.
fn align(a: &[String], b: &[String]) -> (usize, Vec<Op>) {
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j - 1] + cost)
                .min(d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1);
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            if d[i][j] == d[i - 1][j - 1] + cost {
                ops.push(if cost == 0 { Op::Equal(i - 1, j - 1) } else { Op::Substitution(i - 1, j - 1) });
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            ops.push(Op::Deletion(i - 1));
            i -= 1;
        } else {
            ops.push(Op::Insertion(j - 1));
            j -= 1;
        }
    }
    ops.reverse();
    (d[n][m], ops)
}

/// Word Error Rate: Levenshtein over the normalized words / number of
/// reference words. Punctuation and case do NOT count — that's PER and the
/// cap check. Can exceed 1.0 (hypothesis much larger than the reference).
pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let r = normalize_words(reference);
    let h = normalize_words(hypothesis);
    if r.is_empty() {
        // Empty reference: any word in the hypothesis is a total error.
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let (dist, _) = align(&r, &h);
    dist as f64 / r.len() as f64
}

/// Punctuation Error Rate (Picovoice style, marks `{. , ! ?}`).
///
/// Aligns the normalized words (Levenshtein backtrace) and compares the
/// mark adjacent to each word:
/// - aligned pair (equal or substituted) with different marks → 1 error
///   (covers a swapped, missing and spurious mark);
/// - deleted reference word that had a mark → 1 error;
/// - inserted hypothesis word that carries a mark → 1 error (spurious).
///
/// PER = errors / number of marks in the reference. `None` if the reference has
/// no marks at all (a sample without punctuation does not take part in the PER).
///
/// Limitations: content errors contaminate the PER (a wrong word shifts the
/// mark); spurious marks can push the PER above 100%; only the 4 marks
/// in [`PER_MARKS`] count — `…`, `;`, `:` are ignored (see `tokenize`).
pub fn per(reference: &str, hypothesis: &str) -> Option<f64> {
    let rt = tokenize(reference);
    let ht = tokenize(hypothesis);
    let total_marks = rt.iter().filter(|t| t.mark.is_some()).count();
    if total_marks == 0 {
        return None;
    }

    let rw: Vec<String> = rt.iter().map(|t| t.normalized.clone()).collect();
    let hw: Vec<String> = ht.iter().map(|t| t.normalized.clone()).collect();
    let (_, ops) = align(&rw, &hw);

    let mut errors = 0usize;
    for op in ops {
        match op {
            Op::Equal(i, j) | Op::Substitution(i, j) => {
                if rt[i].mark != ht[j].mark {
                    errors += 1;
                }
            }
            Op::Deletion(i) => {
                if rt[i].mark.is_some() {
                    errors += 1;
                }
            }
            Op::Insertion(j) => {
                if ht[j].mark.is_some() {
                    errors += 1;
                }
            }
        }
    }
    Some(errors as f64 / total_marks as f64)
}

/// Capitalization mismatches between pairs of words that are aligned AND equal
/// in normalized form. Returns `(mismatches, total_pairs_compared)`.
/// Substitutions/deletions/insertions are left out — a content error is WER,
/// not case.
pub fn cap_mismatches(reference: &str, hypothesis: &str) -> (usize, usize) {
    let rt = tokenize(reference);
    let ht = tokenize(hypothesis);
    let rw: Vec<String> = rt.iter().map(|t| t.normalized.clone()).collect();
    let hw: Vec<String> = ht.iter().map(|t| t.normalized.clone()).collect();
    let (_, ops) = align(&rw, &hw);

    let mut mismatches = 0usize;
    let mut total = 0usize;
    for op in ops {
        if let Op::Equal(i, j) = op {
            total += 1;
            // Equal normalized ⇒ same characters modulo case; any
            // difference in the original form is a capitalization difference.
            if rt[i].original != ht[j].original {
                mismatches += 1;
            }
        }
    }
    (mismatches, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalize_words ----

    #[test]
    fn normalize_preserves_accents_and_removes_punctuation() {
        assert_eq!(
            normalize_words("Coração, ação — e \u{201C}emoção\u{201D}!"),
            vec!["coração", "ação", "e", "emoção"]
        );
    }

    #[test]
    fn internal_hyphen_becomes_separator() {
        // Documented choice: "guarda-chuva" and "guarda chuva" are equivalent.
        assert_eq!(
            normalize_words("O guarda-chuva ficou lá."),
            vec!["o", "guarda", "chuva", "ficou", "lá"]
        );
    }

    #[test]
    fn multiple_spaces_collapse() {
        assert_eq!(normalize_words("um   dois \n três"), vec!["um", "dois", "três"]);
    }

    // ---- WER ----

    #[test]
    fn wer_zero_for_equal_texts_except_punctuation_and_case() {
        let reference = "Olá, mundo! Tudo bem?";
        let hypothesis = "olá mundo tudo bem";
        assert_eq!(wer(reference, hypothesis), 0.0);
    }

    #[test]
    fn wer_known_value_for_one_substitution_and_one_deletion() {
        // ref: 5 words; "gato"→"rato" (substitution) + "no" dropped (deletion)
        // ⇒ 2 errors / 5 = 0.4.
        let reference = "o gato subiu no telhado";
        let hypothesis = "o rato subiu telhado";
        assert!((wer(reference, hypothesis) - 0.4).abs() < 1e-9);
    }

    #[test]
    fn wer_one_for_empty_reference_with_nonempty_hypothesis() {
        assert_eq!(wer("", "alguma coisa"), 1.0);
        assert_eq!(wer("", ""), 0.0);
    }

    // ---- PER ----

    #[test]
    fn per_zero_for_identical_punctuation() {
        let reference = "Olá, mundo. Tudo certo!";
        let hypothesis = "olá, mundo. tudo certo!";
        assert_eq!(per(reference, hypothesis), Some(0.0));
    }

    #[test]
    fn per_known_value_for_missing_comma() {
        // ref: sim(,) claro(.) — 2 marks. hyp loses the comma ⇒ 1/2 = 0.5.
        let reference = "Sim, claro.";
        let hypothesis = "sim claro.";
        assert_eq!(per(reference, hypothesis), Some(0.5));
    }

    #[test]
    fn per_known_value_for_period_swapped_for_comma() {
        // ref: lá(.) volto(.) — 2 marks. hyp swaps the first period for a
        // comma ⇒ 1/2 = 0.5.
        let reference = "Vou lá. Depois volto.";
        let hypothesis = "vou lá, depois volto.";
        assert_eq!(per(reference, hypothesis), Some(0.5));
    }

    #[test]
    fn per_none_when_reference_has_no_mark() {
        assert_eq!(per("um dois três", "um dois três."), None);
    }

    #[test]
    fn per_counts_mark_of_deleted_word() {
        // ref: oi(,) tudo(?) — 2 marks; "oi," dropped entirely ⇒ 1/2 = 0.5.
        let reference = "Oi, tudo?";
        let hypothesis = "tudo?";
        assert_eq!(per(reference, hypothesis), Some(0.5));
    }

    // ---- cap_mismatches ----

    #[test]
    fn cap_mismatches_basic() {
        // 4 aligned and equal pairs; "O"→"o" and "Brasil"→"brasil" differ.
        let reference = "O Brasil é lindo.";
        let hypothesis = "o brasil é lindo.";
        assert_eq!(cap_mismatches(reference, hypothesis), (2, 4));
    }

    #[test]
    fn cap_mismatches_ignores_substitutions() {
        // "azul"→"verde" is a content error (WER); it doesn't enter the case
        // comparison: only 1 equal pair ("casa") is compared.
        let reference = "casa azul";
        let hypothesis = "casa verde";
        assert_eq!(cap_mismatches(reference, hypothesis), (0, 1));
    }
}
