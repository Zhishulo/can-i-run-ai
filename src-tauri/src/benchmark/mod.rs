pub mod ollama;
pub mod runner;

pub(crate) fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub(crate) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
