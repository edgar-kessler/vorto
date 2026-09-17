//! NVIDIA Parakeet TDT through ONNX Runtime.
//! Pipeline: mel preprocessor (nemo128.onnx) -> Conformer encoder -> TDT greedy decoder.
use anyhow::{bail, Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::{collections::HashMap, path::Path};
use vorto::provider::{
    check_transfer, AudioRequest, Cancellation, ProcessingLocation, ProviderInfo, Transcript,
    TranscriptionProvider,
};

/// Guards against a decoder that keeps emitting tokens on one frame.
const MAX_TOKENS_PER_FRAME: usize = 10;
const STATE: usize = 640;

pub struct Parakeet {
    preprocessor: Session,
    encoder: Session,
    decoder: Session,
    vocab: Vec<String>,
    blank: usize,
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
        Ok(Self {
            preprocessor: session(&dir.join("nemo128.onnx"), 1)?,
            encoder: session(&dir.join("encoder-model.int8.onnx"), threads)?,
            decoder: session(&dir.join("decoder_joint-model.int8.onnx"), 1)?,
            vocab,
            blank,
        })
    }

    fn run(&mut self, samples: &[f32], cancel: &Cancellation) -> Result<String> {
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
        let (encoded, dim, steps, length) = {
            let out = self.encoder.run(ort::inputs![
                "audio_signal" => Value::from_array(([1usize, 128, frames], features))?,
                "length" => Value::from_array(([1usize], vec![feature_len]))?,
            ])?;
            let (shape, encoded) = out["outputs"].try_extract_tensor::<f32>()?;
            let (_, lens) = out["encoded_lengths"].try_extract_tensor::<i64>()?;
            let (dim, steps) = (shape[1] as usize, shape[2] as usize);
            (encoded.to_vec(), dim, steps, (lens[0] as usize).min(steps))
        };

        let mut state1 = vec![0f32; 2 * STATE];
        let mut state2 = vec![0f32; 2 * STATE];
        let mut tokens = Vec::new();
        let mut frame = vec![0f32; dim];
        let (mut t, mut emitted) = (0usize, 0usize);
        while t < length {
            anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
            // Encoder output is [1, dim, T]; gather the column for frame t.
            for (c, value) in frame.iter_mut().enumerate() {
                *value = encoded[c * steps + t];
            }
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
        let text: String = tokens.iter().map(|&t| self.vocab[t].as_str()).collect();
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
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
            text: self.run(&request.samples, &cancel)?,
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
    fn argmax_picks_first_maximum() {
        assert_eq!(argmax(&[0.1, 0.9, 0.9, 0.2]), 1);
    }
}
