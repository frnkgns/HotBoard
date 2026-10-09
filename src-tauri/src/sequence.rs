use std::time::{Duration, Instant};

use crate::model::{Binding, Sequence};

/// Longest gap allowed between two steps of a sequence before the buffer is
/// considered abandoned.
pub const GAP: Duration = Duration::from_millis(1500);

const MAX_STEPS: usize = 6;

/// A key press observed by the hook.
#[derive(Debug, Clone)]
pub struct Press {
    pub binding: Binding,
    pub at: Instant,
}

pub fn label(steps: &Sequence) -> String {
    steps
        .iter()
        .map(crate::shortcut::shortcut_label)
        .collect::<Vec<_>>()
        .join(" then ")
}

/// True when `candidate` is a strict leading run of `other`. Such a pair can
/// never both fire, so one of them must be rejected when saving.
pub fn is_prefix_of(candidate: &Sequence, other: &Sequence) -> bool {
    candidate.len() < other.len()
        && candidate
            .iter()
            .zip(other)
            .all(|(a, b)| a == b)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceCheck {
    pub label: String,
    pub ok: bool,
    pub warning: Option<String>,
    pub error: Option<String>,
}

fn step_error(step: &Binding, index: usize, total: usize) -> Option<String> {
    let check = crate::shortcut::check(step, None);
    if check.ok {
        return None;
    }
    let reason = check
        .error
        .unwrap_or_else(|| "is not a usable key".to_string());
    let position = if total == 1 {
        "This step".to_string()
    } else {
        format!("Step {}", index + 1)
    };
    Some(format!(
        "{position} ({}) {reason}",
        crate::shortcut::shortcut_label(step)
    ))
}

pub fn check(steps: &Sequence, taken_by_other: Option<&str>) -> SequenceCheck {
    let label = label(steps);

    if steps.is_empty() {
        return SequenceCheck {
            label,
            ok: false,
            warning: None,
            error: Some("Record at least one key.".to_string()),
        };
    }

    if let Some(taken) = taken_by_other {
        return SequenceCheck {
            label: label.clone(),
            ok: false,
            warning: None,
            error: Some(format!("{label} is already assigned to \"{taken}\".")),
        };
    }

    for (index, step) in steps.iter().enumerate() {
        if let Some(message) = step_error(step, index, steps.len()) {
            return SequenceCheck {
                label,
                ok: false,
                warning: None,
                error: Some(message),
            };
        }
    }

    let bare_steps = steps
        .iter()
        .filter(|step| !step.ctrl && !step.alt && !step.meta)
        .count();

    let warning = if bare_steps > 0 {
        Some(format!(
            "{label} has {bare_steps} step{} without a modifier, so those keys still type into whatever app is focused. HotBoard plays the sound but never swallows keystrokes.",
            if bare_steps == 1 { "" } else { "s" }
        ))
    } else {
        None
    };

    SequenceCheck {
        label,
        ok: true,
        warning,
        error: None,
    }
}

/// Rolling buffer of recent presses. Reports which sequence, if any, has just
/// been completed. Kept free of Windows types so the matching rules are
/// testable without installing a system-wide hook.
#[derive(Default)]
pub struct Matcher {
    buffer: Vec<Press>,
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    pub fn push(
        &mut self,
        binding: Binding,
        sequences: &[(String, Sequence)],
    ) -> Option<String> {
        let now = Instant::now();

        if self
            .buffer
            .last()
            .map(|last| now.duration_since(last.at) > GAP)
            .unwrap_or(false)
        {
            self.buffer.clear();
        }
        while self.buffer.len() > MAX_STEPS {
            self.buffer.remove(0);
        }

        self.buffer.push(Press { binding, at: now });

        let mut best: Option<(usize, String)> = None;
        for (id, steps) in sequences {
            let len = steps.len();
            if len == 0 || len > self.buffer.len() {
                continue;
            }
            let start = self.buffer.len() - len;
            let matched = self.buffer[start..]
                .iter()
                .zip(steps)
                .all(|(press, step)| press.binding == *step);
            if matched && best.as_ref().map(|(best_len, _)| len > *best_len).unwrap_or(true) {
                best = Some((len, id.clone()));
            }
        }

        let Some((len, id)) = best else { return None };

        if len == self.buffer.len() {
            self.buffer.clear();
        } else {
            self.buffer.drain(0..len);
        }
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(code: &str, ctrl: bool) -> Binding {
        Binding {
            ctrl,
            alt: false,
            shift: false,
            meta: false,
            code: code.to_string(),
        }
    }

    #[test]
    fn matches_a_full_sequence_in_order() {
        let seq: Sequence = vec![b("KeyA", false), b("KeyS", false), b("KeyG", false)];
        let pairs = [("s".to_string(), seq)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyA", false), &pairs), None);
        assert_eq!(m.push(b("KeyS", false), &pairs), None);
        assert_eq!(m.push(b("KeyG", false), &pairs), Some("s".to_string()));
    }

    #[test]
    fn ignores_wrong_order_and_strays() {
        let seq: Sequence = vec![b("KeyA", false), b("KeyS", false)];
        let pairs = [("s".to_string(), seq)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyS", false), &pairs), None);
        assert_eq!(m.push(b("KeyA", false), &pairs), None);
        assert_eq!(m.push(b("KeyG", false), &pairs), None);
    }

    #[test]
    fn modifiers_must_match_each_step() {
        let seq: Sequence = vec![b("KeyK", true), b("KeyS", true)];
        let pairs = [("s".to_string(), seq)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyK", true), &pairs), None);
        assert_eq!(m.push(b("KeyS", true), &pairs), Some("s".to_string()));

        let seq: Sequence = vec![b("KeyK", true), b("KeyS", true)];
        let pairs = [("s".to_string(), seq)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyK", false), &pairs), None);
        assert_eq!(m.push(b("KeyS", true), &pairs), None);
    }

    #[test]
    fn clears_after_a_match_so_the_sequence_can_repeat() {
        let seq: Sequence = vec![b("Digit1", false), b("Digit2", false)];
        let pairs = [("s".to_string(), seq)];
        let mut m = Matcher::new();
        for _ in 0..2 {
            assert_eq!(m.push(b("Digit1", false), &pairs), None);
            assert_eq!(m.push(b("Digit2", false), &pairs), Some("s".to_string()));
        }
    }

    #[test]
    fn fires_the_first_sequence_that_completes() {
        // A and B complete the short sequence, so it fires straight away.
        // Firing immediately keeps latency low; the longer sequence is
        // rejected at save time by `is_prefix_of` because it is ambiguous.
        let short: Sequence = vec![b("KeyA", false), b("KeyB", false)];
        let long: Sequence = vec![b("KeyA", false), b("KeyB", false), b("KeyC", false)];
        let pairs = [("short".to_string(), short), ("long".to_string(), long)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyA", false), &pairs), None);
        assert_eq!(m.push(b("KeyB", false), &pairs), Some("short".to_string()));
        assert_eq!(m.push(b("KeyC", false), &pairs), None);
    }

    #[test]
    fn longer_wins_when_shorter_is_not_also_configured() {
        let long: Sequence = vec![b("KeyA", false), b("KeyB", false), b("KeyC", false)];
        let pairs = [("long".to_string(), long)];
        let mut m = Matcher::new();
        assert_eq!(m.push(b("KeyA", false), &pairs), None);
        assert_eq!(m.push(b("KeyB", false), &pairs), None);
        assert_eq!(m.push(b("KeyC", false), &pairs), Some("long".to_string()));
    }

    #[test]
    fn detects_ambiguous_prefix_sequences() {
        let short: Sequence = vec![b("KeyA", false), b("KeyB", false)];
        let long: Sequence = vec![b("KeyA", false), b("KeyB", false), b("KeyC", false)];
        let same: Sequence = vec![b("KeyA", false), b("KeyB", false)];

        assert!(is_prefix_of(&short, &long));
        assert!(!is_prefix_of(&long, &short));
        assert!(!is_prefix_of(&short, &same));
        assert!(!is_prefix_of(&short, &vec![b("KeyZ", false), b("KeyB", false)]));
    }

    #[test]
    fn rejects_reserved_steps_and_empty_sequences() {
        let reserved: Sequence = vec![b("KeyA", false), b("KeyC", true)];
        let result = check(&reserved, None);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("Step 2"));

        assert!(!check(&Vec::new(), None).ok);
    }

    #[test]
    fn warns_when_steps_have_no_modifier() {
        let bare: Sequence = vec![b("Digit1", false), b("Digit2", false)];
        assert!(check(&bare, None).warning.unwrap().contains("never swallows"));

        let modified: Sequence = vec![b("KeyK", true), b("KeyS", true)];
        assert!(check(&modified, None).warning.is_none());
    }

    #[test]
    fn labels_sequences_readably() {
        let seq: Sequence = vec![b("KeyK", true), b("KeyS", true)];
        assert_eq!(label(&seq), "Ctrl+K then Ctrl+S");
    }
}