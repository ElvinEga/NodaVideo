use super::*;

#[test]
fn generates_waveform() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let source = if root.join(".testdata/super-long.mp4").exists() {
        root.join(".testdata/super-long.mp4")
    } else if root.join("data/tests/super long.mp4").exists() {
        root.join("data/tests/super long.mp4")
    } else {
        println!("Skipping waveform test: test media not present");
        return;
    };
    let waveform = generate_waveform(&source).unwrap();

    assert!(waveform.sample_rate > 0);
    assert!(waveform.total_samples > 0);
    assert!(waveform.levels.len() > 1);
    assert_eq!(waveform.levels[0].samples_per_peak, 64);
    assert_eq!(waveform.columns(0.0, 1.0, 320).len(), 320);
}
