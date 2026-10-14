//! Scratch phase timer for the STEP import path (not shipped, outside the repo).
use std::time::Instant;

fn rss() -> String {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .filter(|l| l.starts_with("VmRSS") || l.starts_with("VmHWM") || l.starts_with("VmPeak"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().cloned().unwrap_or_default();
    let t = Instant::now();
    let bytes = std::fs::read(&path).expect("read");
    println!("read           {:8.1} ms  {} bytes  {}", t.elapsed().as_secs_f64() * 1e3, bytes.len(), rss());
    let text = String::from_utf8_lossy(&bytes).into_owned();
    println!("utf8           {:8.1} ms  {}", t.elapsed().as_secs_f64() * 1e3, rss());

    let t = Instant::now();
    let n = solidcraft_engine::kernel::step_validate(&text);
    println!("step_validate  {:8.1} ms  entities={:?}  {}", t.elapsed().as_secs_f64() * 1e3, n, rss());

    let t = Instant::now();
    let imp = solidcraft_engine::kernel::step_import_shared(&text).expect("import");
    println!("step_import    {:8.1} ms  bodies={}  {}", t.elapsed().as_secs_f64() * 1e3, imp.bodies.len(), rss());

    let faces: usize = imp.bodies.iter().map(|b| b.file_faces).sum();
    println!("file_faces     {faces}");
    println!("after import   live {}", rss());
    std::hint::black_box(&imp);
}
