//! Allocation probe: counting global allocator + scenario runner (scratch tool, not shipped).
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static BIGGEST: AtomicUsize = AtomicUsize::new(0);
static BIGGEST_COUNT: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let live = LIVE.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            TOTAL_ALLOC_BYTES.fetch_add(l.size(), Ordering::Relaxed);
            if l.size() >= 1 << 20 {
                BIGGEST.fetch_max(l.size(), Ordering::Relaxed);
                BIGGEST_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        FREES.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, new) };
        if !q.is_null() {
            if new >= l.size() {
                let live = LIVE.fetch_add(new - l.size(), Ordering::Relaxed) + (new - l.size());
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(l.size() - new, Ordering::Relaxed);
            }
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            if new > l.size() {
                TOTAL_ALLOC_BYTES.fetch_add(new - l.size(), Ordering::Relaxed);
            }
        }
        q
    }
}

#[global_allocator]
static A: Counting = Counting;

fn mark(label: &str) {
    println!(
        "MARK {label}: live={:.2}MB peak={:.2}MB allocs={} frees={} total_alloc={:.1}MB",
        LIVE.load(Ordering::Relaxed) as f64 / 1048576.0,
        PEAK.load(Ordering::Relaxed) as f64 / 1048576.0,
        ALLOCS.load(Ordering::Relaxed),
        FREES.load(Ordering::Relaxed),
        TOTAL_ALLOC_BYTES.load(Ordering::Relaxed) as f64 / 1048576.0,
    );
    let big = BIGGEST.load(Ordering::Relaxed);
    if big > 0 {
        println!("  (largest single allocation {:.2}MB, {} allocations >= 1MB)", big as f64 / 1048576.0, BIGGEST_COUNT.load(Ordering::Relaxed));
    }
}

fn report(session: &solidcraft_engine::Session, label: &str) {
    let (mut bodies, mut sketches, mut states, mut n) = (0usize, 0usize, 0usize, 0usize);
    let (mut sheets, mut threads, mut construct) = (0usize, 0usize, 0usize);
    for r in &session.model.results {
        if r.skipped {
            continue;
        }
        let st = &r.state;
        n += 1;
        states += st.retained_bytes();
        bodies += st.bodies.iter().map(|b| b.body.retained_bytes()).sum::<usize>();
        // 0.10.0 keeps a snapshot's sketches behind Arc, so the method needs a deref through it.
        sketches += st.sketches.iter().map(|s| solidcraft_doc::SolvedSketch::retained_bytes(s)).sum::<usize>();
        sheets += st.sheets.len();
        threads += st.threads.len();
        construct += st.construct.len();
    }
    let last = session.model.state();
    let last_sketches: usize = last.sketches.iter().map(|s| solidcraft_doc::SolvedSketch::retained_bytes(s)).sum();
    println!(
        "REPORT {label}: states={n} sum_retained={:.1}MB bodies={:.1}MB sketches={:.1}MB sheets={sheets} threads={threads} construct={construct} | last retained={:.1}MB bodies={} last_sketch_bytes={:.1}MB",
        states as f64 / 1048576.0,
        bodies as f64 / 1048576.0,
        sketches as f64 / 1048576.0,
        last.retained_bytes() as f64 / 1048576.0,
        last.bodies.len(),
        last_sketches as f64 / 1048576.0
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = &args[0];
    let no_cache = args.iter().any(|a| a == "--no-cache");
    let drop_model = args.iter().any(|a| a == "--drop-model");
    let drop_session = args.iter().any(|a| a == "--drop-session");
    let rollback = args.iter().any(|a| a == "--rollback");

    let text = std::fs::read_to_string(path).expect("read design");
    let script: serde_json::Value = serde_json::from_str(&text).expect("parse script");

    if args.iter().any(|a| a == "--dump-samples") {
        let dir = std::path::Path::new("perf/designs/samples");
        let _ = std::fs::create_dir_all(dir);
        for s in solidcraft_engine::sample::samples() {
            let path = dir.join(format!("{}.json", s.name.replace(' ', "_")));
            std::fs::write(&path, serde_json::to_string_pretty(&(s.script)()).expect("json")).expect("write");
            println!("SAMPLE {} -> {}", s.name, path.display());
        }
    }
    mark("start");
    let mut session = Some(solidcraft_engine::Session::default());
    if no_cache {
        if let Some(s) = session.as_mut() {
            s.model.no_cache = true;
        }
    }
    mark("session");
    if let Some(s) = session.as_mut() {
        s.run_script(&script).expect("run script");
    }
    mark("evaluated");
    if args.iter().any(|a| a == "--report") {
        if let Some(s) = session.as_ref() {
            report(s, "evaluated");
        }
    }
    if args.iter().any(|a| a == "--drop-result-states") {
        // Replace each result's state (except the last) with an empty one: measures what the
        // timeline's own snapshots hold, apart from anything the naming chain keeps alive.
        if let Some(s) = session.as_mut() {
            let last = s.model.results.iter().rposition(|r| !r.skipped);
            for (i, r) in s.model.results.iter_mut().enumerate() {
                if Some(i) == last {
                    continue;
                }
                r.state = std::sync::Arc::new(solidcraft_doc::ModelState::default());
            }
        }
        mark("dropped-result-states");
        if let Some(s) = session.as_ref() {
            report(s, "dropped-result-states");
        }
    }
    if args.iter().any(|a| a == "--drop-sketch-copies") {
        // Keep only the last state's sketches, drop the copies earlier snapshots hold.
        if let Some(s) = session.as_mut() {
            let last = s.model.results.iter().rposition(|r| !r.skipped);
            for (i, r) in s.model.results.iter_mut().enumerate() {
                if Some(i) == last {
                    continue;
                }
                let mut st = (*r.state).clone();
                st.sketches.clear();
                r.state = std::sync::Arc::new(st);
            }
        }
        mark("dropped-sketch-copies");
        if let Some(s) = session.as_ref() {
            report(s, "dropped-sketch-copies");
        }
    }
    if args.iter().any(|a| a == "--dismantle") {
        // Step through the holders one at a time, printing what each step frees.
        if let Some(s) = session.as_mut() {
            s.undo.clear();
            s.redo.clear();
            s.clipboard.clear();
            mark("d-undo-clipboard");
            let fresh = solidcraft_doc::Model::new();
            s.model.cache = fresh.cache;
            mark("d-cache");
            s.model.results.clear();
            mark("d-results");
            s.model = solidcraft_doc::Model::new();
            mark("d-model");
            s.doc = std::sync::Arc::new(solidcraft_doc::Document::new("empty"));
            mark("d-doc");
        }
        session = None;
        mark("d-session");
    }
    if let Some(n) = args.iter().position(|a| a == "--clear-state-parts") {
        // Walk the kept states BACKWARD, replacing each with a copy that has the named parts
        // emptied: a state cleared this way releases the references later states' naming
        // origins held, so the snapshot actually goes away and the allocator sees the drop.
        let what = args.get(n + 1).cloned().unwrap_or_else(|| "all".into());
        if let Some(s) = session.as_mut() {
            let count = s.model.results.len();
            for i in (0..count).rev() {
                let mut st = (*s.model.results[i].state).clone();
                if what == "bodies" || what == "all" {
                    st.bodies.clear();
                }
                if what == "sketches" || what == "all" {
                    st.sketches.clear();
                }
                if what == "construct" {
                    st.construct.clear();
                    st.sheets.clear();
                    st.threads.clear();
                }
                s.model.results[i].state = std::sync::Arc::new(st);
            }
        }
        mark(&format!("cleared-state-{what}"));
    }
    if args.iter().any(|a| a == "--drop-state-bodies") {
        // Free every retained snapshot's bodies (whoever still references the snapshot).
        if let Some(s) = session.as_mut() {
            for r in s.model.results.iter_mut() {
                let mut st = (*r.state).clone();
                st.bodies.clear();
                r.state = std::sync::Arc::new(st);
            }
        }
        mark("dropped-state-bodies");
    }
    if args.iter().any(|a| a == "--drop-mesh-cells") {
        // Rebuild each retained body without its cached display mesh.
        if let Some(s) = session.as_mut() {
            for r in s.model.results.iter_mut() {
                let mut st = (*r.state).clone();
                let fresh: Vec<solidcraft_doc::ModelBody> = st
                    .bodies
                    .iter()
                    .map(|b| solidcraft_doc::ModelBody::new(b.name.clone(), b.body.clone(), b.feature))
                    .collect();
                st.bodies = fresh;
                r.state = std::sync::Arc::new(st);
            }
        }
        mark("dropped-mesh-cells");
    }
    if args.iter().any(|a| a == "--clear-cache") {
        if let Some(s) = session.as_mut() {
            let fresh = solidcraft_doc::Model::new();
            s.model.cache = fresh.cache;
        }
    }
    if args.iter().any(|a| a == "--clear-sketches") {
        if let Some(s) = session.as_mut() {
            for r in s.model.results.iter_mut() {
                let st = std::sync::Arc::make_mut(&mut r.state);
                st.threads.clear();
                st.sketches.clear();
                st.sheets.clear();
                st.construct.clear();
            }
        }
    }
    if args.iter().any(|a| a == "--clear-results") {
        if let Some(s) = session.as_mut() {
            s.model.results.clear();
        }
    }
    if args.iter().any(|a| a == "--strip-states") {
        if let Some(s) = session.as_mut() {
            let last = s.model.results.iter().rposition(|r| !r.skipped);
            for (i, r) in s.model.results.iter_mut().enumerate() {
                if Some(i) == last {
                    continue;
                }
                let st = std::sync::Arc::make_mut(&mut r.state);
                st.bodies.clear();
                st.threads.clear();
                st.sketches.clear();
                st.sheets.clear();
                st.construct.clear();
            }
        }
    }
    if args.iter().any(|a| a == "--strip-meshes") {
        if let Some(s) = session.as_mut() {
            for r in s.model.results.iter_mut() {
                let st = std::sync::Arc::make_mut(&mut r.state);
                // Rebuild each body with a fresh (unwarmed) mesh cell.
                let fresh: Vec<solidcraft_doc::ModelBody> = st
                    .bodies
                    .iter()
                    .map(|b| solidcraft_doc::ModelBody::new(b.name.clone(), b.body.clone(), b.feature))
                    .collect();
                st.bodies = fresh;
            }
        }
    }
    if args.iter().any(|a| a == "--strip-undo") {
        if let Some(s) = session.as_mut() {
            s.undo.clear();
            s.redo.clear();
        }
    }
    if args.iter().any(|a| a == "--strip-doc") {
        if let Some(s) = session.as_mut() {
            s.doc = std::sync::Arc::new(solidcraft_doc::Document::new("empty"));
        }
    }
    if args.iter().any(|a| a == "--strip-sketches-only") {
        if let Some(s) = session.as_mut() {
            for r in s.model.results.iter_mut() {
                let st = std::sync::Arc::make_mut(&mut r.state);
                st.sketches.clear();
            }
        }
    }
    if args.iter().any(|a| a == "--trace-commands") {
        if let Some(s) = session.as_mut() {
            let cmds = script.get("commands").and_then(|c| c.as_array()).cloned().unwrap_or_default();
            for (i, c) in cmds.iter().enumerate() {
                let id = c.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let params = c.get("params").cloned().unwrap_or(serde_json::Value::Null);
                let b0 = LIVE.load(Ordering::Relaxed);
                let a0 = ALLOCS.load(Ordering::Relaxed);
                let t0 = std::time::Instant::now();
                let r = s.execute(id, &params);
                let dt = t0.elapsed().as_secs_f64();
                let live = LIVE.load(Ordering::Relaxed);
                println!(
                    "TRACE {i:3} {id:28} dt={dt:7.3}s live={:8.2}MB delta={:+9.2}MB allocs={} err={}",
                    live as f64 / 1048576.0,
                    (live as i64 - b0 as i64) as f64 / 1048576.0,
                    ALLOCS.load(Ordering::Relaxed) - a0,
                    r.is_err()
                );
            }
        }
    }
    if let Some(n) = args.iter().position(|a| a == "--cycle-param") {
        let name = args.get(n + 1).cloned().unwrap_or_else(|| "pitch".into());
        let rounds: usize = args.get(n + 2).and_then(|x| x.parse().ok()).unwrap_or(10);
        if let Some(s) = session.as_mut() {
            mark("before-cycles");
            for i in 0..rounds {
                let v = 14.0 + (i as f64) * 0.25;
                let r = s.execute("parameters.change", &serde_json::json!({"name": name, "expression": format!("{v} mm")}));
                let live = LIVE.load(Ordering::Relaxed);
                println!(
                    "CYCLE {i:3} {name}={v} live={:8.2}MB peak={:8.2}MB allocs={} err={}",
                    live as f64 / 1048576.0,
                    PEAK.load(Ordering::Relaxed) as f64 / 1048576.0,
                    ALLOCS.load(Ordering::Relaxed),
                    r.is_err()
                );
            }
            mark("after-cycles");
        }
    }
    if args.iter().any(|a| a == "--clear-cache-now") {
        if let Some(s) = session.as_mut() {
            let fresh = solidcraft_doc::Model::new();
            s.model.cache = fresh.cache;
        }
        mark("cache-cleared");
    }
    mark("decomposed");
    if let Some(n) = args.iter().position(|a| a == "--clone") {
        let count: usize = args.get(n + 1).and_then(|x| x.parse().ok()).unwrap_or(100);
        if let Some(s) = session.as_mut() {
            let st = s.model.state();
            let b0 = LIVE.load(Ordering::Relaxed);
            let a0 = ALLOCS.load(Ordering::Relaxed);
            let t0 = std::time::Instant::now();
            let mut keep = Vec::new();
            for _ in 0..count {
                keep.push((*st).clone());
            }
            let dt = t0.elapsed().as_secs_f64();
            let bytes = LIVE.load(Ordering::Relaxed).saturating_sub(b0);
            println!(
                "CLONE n={count} per_clone_bytes={} per_clone_allocs={} total_small={dt:.3}",
                bytes / count as usize,
                (ALLOCS.load(Ordering::Relaxed) - a0) / count,
            );
            std::hint::black_box(&keep);
        }
    }
    mark("cloned");
    if rollback {
        if let Some(s) = session.as_mut() {
            let features: Vec<u64> = s.doc.features.iter().map(|f| f.id).collect();
            println!("FEATURES {}", features.len());
            let mid = features[features.len() / 2];
            s.execute("timeline.rollback", &serde_json::json!({"feature": mid})).ok();
            mark("rolled-back-half");
            s.execute("timeline.rollback", &serde_json::json!({"feature": features[0]})).ok();
            mark("rolled-back-first");
        }
    }
    if drop_model {
        let doc = session.as_ref().map(|s| s.doc.clone());
        session = None;
        mark("dropped-session");
        if let Some(doc) = doc {
            let mut m = solidcraft_doc::Model::new();
            m.evaluate(&doc);
            mark("reevaluated-post-drop");
            std::hint::black_box(&m);
        }
    }
    if drop_session {
        session = None;
        mark("dropped");
    }
    let dir = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in dir.lines().filter(|l| l.starts_with("VmRSS") || l.starts_with("VmHWM")) {
        println!("PROC {line}");
    }
    std::hint::black_box(&session);
}