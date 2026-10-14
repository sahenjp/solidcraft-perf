//! Scratch: how much of a STEP import is the per-solid tessellation Body::new throws away?
use std::time::Instant;

fn rss() -> String {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines().filter(|l| l.starts_with("VmRSS") || l.starts_with("VmHWM")).collect::<Vec<_>>().join(" ")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().cloned().unwrap_or_default();
    let bytes = std::fs::read(&path).expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let t = Instant::now();
    let imp = solvecraft_engine::kernel::step_import_shared(&text).expect("import");
    println!("step_import    {:8.1} ms  bodies={}  {}", t.elapsed().as_secs_f64() * 1e3, imp.bodies.len(), rss());

    // Same tolerance Body::new uses internally: (span*0.01).clamp(1e-3, 1.0).
    let t = Instant::now();
    let mut tris = 0usize;
    for b in &imp.bodies {
        let tol = (b.body.size() * 0.01).clamp(1e-3, 1.0);
        if let Ok(m) = b.body.tessellate(tol) {
            tris += m.triangles.len();
        }
    }
    println!("tessellate all {:8.1} ms  triangles={}  {}", t.elapsed().as_secs_f64() * 1e3, tris, rss());
    println!("bodies {} file_faces {}", imp.bodies.len(), imp.bodies.iter().map(|b| b.file_faces).sum::<usize>());
}
