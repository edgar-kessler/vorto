//! NVIDIA Parakeet TDT through ONNX Runtime.
//! Pipeline: mel preprocessor (nemo128.onnx) -> Conformer encoder -> TDT greedy decoder, then
//! the user's terms are checked against the audio: the same frames are scored once as the words
//! the decoder wrote and once as the term, so a term never goes in just because it looks similar.
use anyhow::{bail, Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::{collections::HashMap, path::Path};
use vorto::provider::{
    check_transfer, AudioRequest, Cancellation, ProcessingLocation, ProviderInfo, Transcript,
    TranscriptionProvider,
};
use vorto::vocab::{self, Term};

/// Guards against a decoder that keeps emitting tokens on one frame.
const MAX_TOKENS_PER_FRAME: usize = 10;
const STATE: usize = 640;
/// How much the audio may favor the written words per piece of the term, in nats, and the term
/// still wins. A word the user has corrected before gets more: they told us it's likely.
const BOOST: f32 = 2.0;
const BOOST_HEARD_BEFORE: f32 = 5.0;
/// Longest term that gets scored, in vocabulary pieces.
const MAX_PIECES: usize = 12;
/// Longest stretch of audio scored for one candidate: about three seconds.
const MAX_FRAMES: usize = 40;

/// The decoder's memory just before a word begins.
#[derive(Clone)]
struct Predictor {
    last: i32,
    state1: Vec<f32>,
    state2: Vec<f32>,
}

/// A word the decoder wrote, with where it starts in the audio.
struct Word {
    text: String,
    frame: usize,
    before: Predictor,
    pieces: Vec<usize>,
}

/// The encoder's output for one recording.
struct Encoded {
    data: Vec<f32>,
    dim: usize,
    steps: usize,
    length: usize,
}
impl Encoded {
    /// Encoder output is [1, dim, T]; gathers the column for frame `t`.
    fn frame(&self, t: usize, out: &mut [f32]) {
        for (c, value) in out.iter_mut().enumerate() {
            *value = self.data[c * self.steps + t];
        }
    }
}

pub struct Parakeet {
    preprocessor: Session,
    encoder: Session,
    decoder: Session,
    vocab: Vec<String>,
    blank: usize,
    /// Vocabulary pieces to their ids, with the word boundary as a space.
    pieces: HashMap<String, usize>,
}

fn session(path: &Path, threads: usize) -> Result<Session> {
    let error = |e: ort::Error<_>| anyhow::anyhow!("{e}");
    Session::builder()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(error)?
        .with_intra_threads(threads)
        .map_err(error)?
        .commit_from_file(path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("Could not open {}", path.display()))
}

impl Parakeet {
    pub fn load(dir: &Path, threads: usize) -> Result<Self> {
        let vocab = load_vocab(&dir.join("vocab.txt"))?;
        if vocab.len() < 32 {
            bail!("The model vocabulary is damaged. Repair the download.");
        }
        let blank = vocab
            .iter()
            .position(|t| t == "<blk>")
            .unwrap_or(vocab.len() - 1);
        // Exports name the encoder differently.
        let encoder = ["encoder-model.int8.onnx", "encoder.int8.onnx"]
            .map(|name| dir.join(name))
            .into_iter()
            .find(|path| path.exists())
            .context("The voice model is incomplete. Repair the download.")?;
        let pieces = vocab
            .iter()
            .enumerate()
            .map(|(i, piece)| (piece.clone(), i))
            .collect();
        Ok(Self {
            preprocessor: session(&dir.join("nemo128.onnx"), 1)?,
            encoder: session(&encoder, threads)?,
            decoder: session(&dir.join("decoder_joint-model.int8.onnx"), 1)?,
            vocab,
            blank,
            pieces,
        })
    }

    fn run(&mut self, samples: &[f32], terms: &[Term], cancel: &Cancellation) -> Result<String> {
        let n = samples.len();
        let (features, frames, feature_len) = {
            let out = self.preprocessor.run(ort::inputs![
                "waveforms" => Value::from_array(([1usize, n], samples.to_vec()))?,
                "waveforms_lens" => Value::from_array(([1usize], vec![n as i64]))?,
            ])?;
            let (shape, features) = out["features"].try_extract_tensor::<f32>()?;
            let (_, lens) = out["features_lens"].try_extract_tensor::<i64>()?;
            (features.to_vec(), shape[2] as usize, lens[0])
        };
        anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
        let encoded = {
            let out = self.encoder.run(ort::inputs![
                "audio_signal" => Value::from_array(([1usize, 128, frames], features))?,
                "length" => Value::from_array(([1usize], vec![feature_len]))?,
            ])?;
            let (shape, encoded) = out["outputs"].try_extract_tensor::<f32>()?;
            let (_, lens) = out["encoded_lengths"].try_extract_tensor::<i64>()?;
            let (dim, steps) = (shape[1] as usize, shape[2] as usize);
            Encoded {
                data: encoded.to_vec(),
                dim,
                steps,
                length: (lens[0] as usize).min(steps),
            }
        };
        let (dim, length) = (encoded.dim, encoded.length);

        let mut state1 = vec![0f32; 2 * STATE];
        let mut state2 = vec![0f32; 2 * STATE];
        let mut tokens: Vec<usize> = Vec::new();
        let mut words: Vec<Word> = Vec::new();
        let mut frame = vec![0f32; dim];
        let (mut t, mut emitted) = (0usize, 0usize);
        while t < length {
            anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
            encoded.frame(t, &mut frame);
            let last = *tokens.last().unwrap_or(&self.blank) as i32;
            let out = self.decoder.run(ort::inputs![
                "encoder_outputs" => Value::from_array(([1usize, dim, 1], frame.clone()))?,
                "targets" => Value::from_array(([1usize, 1], vec![last]))?,
                "target_length" => Value::from_array(([1usize], vec![1i32]))?,
                "input_states_1" => Value::from_array(([2usize, 1, STATE], state1.clone()))?,
                "input_states_2" => Value::from_array(([2usize, 1, STATE], state2.clone()))?,
            ])?;
            let (_, logits) = out["outputs"].try_extract_tensor::<f32>()?;
            let vocab = self.vocab.len();
            let token = argmax(&logits[..vocab]);
            // TDT appends duration logits after the vocabulary.
            let skip = if logits.len() > vocab {
                argmax(&logits[vocab..])
            } else {
                0
            };
            if token != self.blank {
                if !terms.is_empty() {
                    let piece = &self.vocab[token];
                    if piece.starts_with(' ') || words.is_empty() {
                        words.push(Word {
                            text: String::new(),
                            frame: t,
                            before: Predictor {
                                last,
                                state1: state1.clone(),
                                state2: state2.clone(),
                            },
                            pieces: Vec::new(),
                        });
                    }
                    if let Some(word) = words.last_mut() {
                        word.text.push_str(piece);
                        word.pieces.push(token);
                    }
                }
                let (_, s1) = out["output_states_1"].try_extract_tensor::<f32>()?;
                let (_, s2) = out["output_states_2"].try_extract_tensor::<f32>()?;
                state1.copy_from_slice(s1);
                state2.copy_from_slice(s2);
                tokens.push(token);
                emitted += 1;
            }
            if skip > 0 {
                t += skip;
                emitted = 0;
            } else if token == self.blank || emitted == MAX_TOKENS_PER_FRAME {
                t += 1;
                emitted = 0;
            }
        }
        if !terms.is_empty() && !words.is_empty() {
            return self.check_terms(&encoded, &words, terms, cancel);
        }
        let text: String = tokens.iter().map(|&t| self.vocab[t].as_str()).collect();
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// Swaps a term in wherever the audio supports it about as well as the words written there.
    fn check_terms(
        &mut self,
        encoded: &Encoded,
        words: &[Word],
        terms: &[Term],
        cancel: &Cancellation,
    ) -> Result<String> {
        let texts: Vec<&str> = words.iter().map(|w| w.text.trim()).collect();
        let found = vocab::candidates(&texts, terms);
        let mut chosen: Vec<vocab::Candidate> = Vec::new();
        for candidate in found {
            anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
            let term = &terms[candidate.term];
            let Some(wanted) = tokenize(&format!(" {}", term.term), &self.pieces) else {
                continue;
            };
            if wanted.len() > MAX_PIECES {
                continue;
            }
            let first = &words[candidate.word];
            let end = candidate.word + candidate.len;
            let start = first.frame.saturating_sub(1);
            let stop = words
                .get(end)
                .map_or(encoded.length, |next| next.frame)
                .clamp(start + 1, encoded.length)
                .min(start + MAX_FRAMES);
            let heard: Vec<usize> = words[candidate.word..end]
                .iter()
                .flat_map(|w| w.pieces.iter().copied())
                .collect();
            if heard == wanted {
                continue;
            }
            let as_heard = self.score(encoded, &heard, &first.before, start, stop)?;
            let as_term = self.score(encoded, &wanted, &first.before, start, stop)?;
            let plain = |s: &str| s.to_lowercase().replace(' ', "");
            let joined = plain(&texts[candidate.word..end].concat());
            let before = term
                .sounds_like
                .iter()
                .any(|s| joined.contains(&plain(s)) && !s.is_empty());
            let boost = if before { BOOST_HEARD_BEFORE } else { BOOST };
            let wins = as_term + boost * wanted.len() as f32 >= as_heard;
            if std::env::var_os("VORTO_DEBUG_TERMS").is_some() {
                eprintln!(
                    "{:?} as heard {as_heard:.2}, {:?} {as_term:.2}, {} pieces, boost {boost}: {wins}",
                    &texts[candidate.word..end],
                    term.term,
                    wanted.len()
                );
            }
            if wins {
                chosen.push(candidate);
            }
        }
        // From the back, so the positions of earlier candidates stay right.
        let mut out: Vec<String> = texts.iter().map(|w| w.to_string()).collect();
        for candidate in chosen.iter().rev() {
            let refs: Vec<&str> = out.iter().map(String::as_str).collect();
            out = vocab::swap(&refs, candidate, &terms[candidate.term].term);
        }
        Ok(out.join(" "))
    }

    /// How well the audio supports these pieces: the best order-keeping assignment of pieces to
    /// frames, as a sum of log probabilities. Higher is better.
    fn score(
        &mut self,
        encoded: &Encoded,
        pieces: &[usize],
        before: &Predictor,
        start: usize,
        stop: usize,
    ) -> Result<f32> {
        let frames = stop.saturating_sub(start);
        if pieces.is_empty() || frames == 0 {
            return Ok(f32::NEG_INFINITY);
        }
        let mut predictor = before.clone();
        let mut frame = vec![0f32; encoded.dim];
        // The best sum so far for the pieces before this one, ending at or before each frame.
        let mut best = vec![0f32; frames];
        for &piece in pieces {
            let mut scored = vec![f32::NEG_INFINITY; frames];
            let mut next_state = None;
            for (f, slot) in scored.iter_mut().enumerate() {
                encoded.frame(start + f, &mut frame);
                let out = self.decoder.run(ort::inputs![
                    "encoder_outputs" => Value::from_array(([1usize, encoded.dim, 1], frame.clone()))?,
                    "targets" => Value::from_array(([1usize, 1], vec![predictor.last]))?,
                    "target_length" => Value::from_array(([1usize], vec![1i32]))?,
                    "input_states_1" => Value::from_array(([2usize, 1, STATE], predictor.state1.clone()))?,
                    "input_states_2" => Value::from_array(([2usize, 1, STATE], predictor.state2.clone()))?,
                ])?;
                let (_, logits) = out["outputs"].try_extract_tensor::<f32>()?;
                *slot = best[f] + log_softmax(&logits[..self.vocab.len()], piece);
                if next_state.is_none() {
                    let (_, s1) = out["output_states_1"].try_extract_tensor::<f32>()?;
                    let (_, s2) = out["output_states_2"].try_extract_tensor::<f32>()?;
                    next_state = Some((s1.to_vec(), s2.to_vec()));
                }
            }
            // The next piece may land on this frame or any later one.
            let mut running = f32::NEG_INFINITY;
            for (slot, value) in best.iter_mut().zip(&scored) {
                running = running.max(*value);
                *slot = running;
            }
            if let Some((s1, s2)) = next_state {
                predictor = Predictor {
                    last: piece as i32,
                    state1: s1,
                    state2: s2,
                };
            }
        }
        Ok(best[frames - 1])
    }
}

impl TranscriptionProvider for Parakeet {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "parakeet-local".into(),
            label: "Local Parakeet".into(),
            location: ProcessingLocation::OnDevice,
        }
    }
    /// Parakeet detects the language itself; `request.language` is ignored.
    fn transcribe(&mut self, request: AudioRequest, cancel: Cancellation) -> Result<Transcript> {
        check_transfer(&self.info(), &request)?;
        Ok(Transcript {
            text: self.run(&request.samples, &request.terms, &cancel)?,
            language: None,
        })
    }
}

fn load_vocab(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path).context("The model vocabulary is missing")?;
    let mut map = HashMap::new();
    for line in text.lines() {
        let Some((piece, id)) = line.rsplit_once(' ') else {
            continue;
        };
        map.insert(id.trim().parse::<usize>()?, piece.replace('\u{2581}', " "));
    }
    (0..map.len())
        .map(|i| map.remove(&i).context("The model vocabulary is incomplete"))
        .collect()
}

/// Splits text into the fewest vocabulary pieces, trying the term as written. `None` when a
/// letter has no piece, as for scripts the model doesn't know.
fn tokenize(text: &str, pieces: &HashMap<String, usize>) -> Option<Vec<usize>> {
    let chars: Vec<char> = text.chars().collect();
    let mut best: Vec<Option<(usize, usize, usize)>> = vec![None; chars.len() + 1];
    best[0] = Some((0, 0, 0));
    for start in 0..chars.len() {
        let Some((count, _, _)) = best[start] else {
            continue;
        };
        for end in start + 1..=chars.len().min(start + 16) {
            let piece: String = chars[start..end].iter().collect();
            if let Some(&id) = pieces.get(&piece) {
                if best[end].is_none_or(|(c, _, _)| count + 1 < c) {
                    best[end] = Some((count + 1, start, id));
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut at = chars.len();
    while at > 0 {
        let (_, start, id) = best[at]?;
        out.push(id);
        at = start;
    }
    out.reverse();
    Some(out)
}

/// The log probability of one token among all of them.
fn log_softmax(logits: &[f32], token: usize) -> f32 {
    let top = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum: f32 = logits.iter().map(|&v| (v - top).exp()).sum();
    logits[token] - top - sum.ln()
}

fn argmax(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .fold(0, |best, (i, &v)| if v > values[best] { i } else { best })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vocabulary_maps_word_boundaries_to_spaces() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("vocab.txt");
        std::fs::write(&p, "<unk> 0\n\u{2581}hello 1\nworld 2\n").unwrap();
        assert_eq!(load_vocab(&p).unwrap(), ["<unk>", " hello", "world"]);
    }
    #[test]
    fn splits_a_term_into_the_fewest_pieces() {
        let pieces: HashMap<String, usize> = [
            (" V", 0),
            ("or", 1),
            ("to", 2),
            ("o", 3),
            ("r", 4),
            ("t", 5),
            (" Vor", 6),
        ]
        .into_iter()
        .map(|(p, i)| (p.to_string(), i))
        .collect();
        assert_eq!(tokenize(" Vorto", &pieces), Some(vec![6, 2]));
        assert_eq!(tokenize(" Vorto!", &pieces), None);
    }
    #[test]
    fn log_softmax_sums_to_one() {
        let logits = [1.0, 2.0, 3.0];
        let total: f32 = (0..3).map(|t| log_softmax(&logits, t).exp()).sum();
        assert!((total - 1.0).abs() < 1e-4);
    }
    #[test]
    fn argmax_picks_first_maximum() {
        assert_eq!(argmax(&[0.1, 0.9, 0.9, 0.2]), 1);
    }
}
