//! Scratch: does `unmeshed_faces` reuse `Body::new`'s missing-face count, or re-mesh?
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().cloned().unwrap_or_default();
    let bytes = std::fs::read(&path).expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let imp = solidcraft_engine::kernel::step_import_shared(&text).expect("import");
    let t = Instant::now();
    let total: usize = imp.bodies.iter().map(|b| b.body.unmeshed_faces()).sum();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    println!("{path}: unmeshed_faces over {} bodies: {ms:.1} ms (sum {total})  {:.3} ms/body", imp.bodies.len(), ms / imp.bodies.len() as f64);
}
