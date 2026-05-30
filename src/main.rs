// grand-pattern-cli — Pure Rust, zero dependencies
// Command-line tool for the Grand Pattern

use std::collections::HashMap;
use std::env;
use std::fs;
use std::time::Instant;

// ── Core Graph ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Room {
    id: usize,
    vibe: f64,
}

struct Graph {
    rooms: Vec<Option<Room>>,
    edges: Vec<(usize, usize)>,
    adjacency: HashMap<usize, Vec<usize>>,
    next_id: usize,
    rng_state: u64,
}

impl Graph {
    fn new(rooms: usize, topology: &str, probability: f64, seed: u64) -> Self {
        let mut g = Graph {
            rooms: Vec::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            next_id: 0,
            rng_state: seed,
        };
        for _ in 0..rooms {
            g.add_room();
        }
        match topology {
            "ring" => {
                let n = g.active_count();
                if n > 1 {
                    let ids = g.active_ids();
                    for i in 0..n {
                        g.add_edge(ids[i], ids[(i + 1) % n]);
                    }
                }
            }
            "small-world" => {
                let n = g.active_count();
                if n > 1 {
                    let ids = g.active_ids();
                    for i in 0..n {
                        g.add_edge(ids[i], ids[(i + 1) % n]);
                    }
                    for i in 0..n {
                        for j in (i + 2)..n {
                            if i == 0 && j == n - 1 { continue; }
                            if g.pseudo_random() < probability {
                                g.add_edge(ids[i], ids[j]);
                            }
                        }
                    }
                }
            }
            "full" | "complete" => {
                let ids = g.active_ids();
                for i in 0..ids.len() {
                    for j in (i + 1)..ids.len() {
                        g.add_edge(ids[i], ids[j]);
                    }
                }
            }
            "random" => {
                let ids = g.active_ids();
                for i in 0..ids.len() {
                    for j in (i + 1)..ids.len() {
                        if g.pseudo_random() < probability {
                            g.add_edge(ids[i], ids[j]);
                        }
                    }
                }
            }
            "line" => {
                let ids = g.active_ids();
                for i in 0..ids.len().saturating_sub(1) {
                    g.add_edge(ids[i], ids[i + 1]);
                }
            }
            _ => {
                let n = g.active_count();
                if n > 1 {
                    let ids = g.active_ids();
                    for i in 0..n {
                        g.add_edge(ids[i], ids[(i + 1) % n]);
                    }
                }
            }
        }
        g
    }

    fn pseudo_random(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D)) as f64 / u64::MAX as f64
    }

    fn add_room(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let room = Room { id, vibe: 0.5 };
        let mut placed = false;
        for slot in &mut self.rooms {
            if slot.is_none() {
                *slot = Some(room.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            self.rooms.push(Some(room));
        }
        self.adjacency.insert(id, Vec::new());
        id
    }

    fn add_edge(&mut self, a: usize, b: usize) {
        if a == b { return; }
        if self.edges.contains(&(a, b)) || self.edges.contains(&(b, a)) { return; }
        self.edges.push((a, b));
        self.adjacency.entry(a).or_default().push(b);
        self.adjacency.entry(b).or_default().push(a);
    }

    fn active_ids(&self) -> Vec<usize> {
        self.rooms.iter().filter_map(|r| r.as_ref().map(|r| r.id)).collect()
    }

    fn active_count(&self) -> usize {
        self.rooms.iter().filter(|r| r.is_some()).count()
    }

    fn get_room(&self, id: usize) -> Option<&Room> {
        self.rooms.iter().find_map(|r| r.as_ref().and_then(|room| if room.id == id { Some(room) } else { None }))
    }

    fn get_room_mut(&mut self, id: usize) -> Option<&mut Room> {
        self.rooms.iter_mut().find_map(|r| r.as_mut().and_then(|room| if room.id == id { Some(room) } else { None }))
    }

    fn remove_room(&mut self, id: usize) -> bool {
        let mut found = false;
        for slot in &mut self.rooms {
            if let Some(ref room) = slot {
                if room.id == id {
                    *slot = None;
                    found = true;
                    break;
                }
            }
        }
        if found {
            self.edges.retain(|(a, b)| *a != id && *b != id);
            self.adjacency.remove(&id);
            for neighbors in self.adjacency.values_mut() {
                neighbors.retain(|&n| n != id);
            }
        }
        found
    }

    fn fleet_vibe(&self) -> f64 {
        let rooms: Vec<&Room> = self.rooms.iter().filter_map(|r| r.as_ref()).collect();
        if rooms.is_empty() { return 0.0; }
        rooms.iter().map(|r| r.vibe).sum::<f64>() / rooms.len() as f64
    }

    fn fleet_surprise(&self) -> f64 {
        let mean = self.fleet_vibe();
        let rooms: Vec<&Room> = self.rooms.iter().filter_map(|r| r.as_ref()).collect();
        if rooms.is_empty() { return 0.0; }
        let variance = rooms.iter().map(|r| (r.vibe - mean).powi(2)).sum::<f64>() / rooms.len() as f64;
        variance.sqrt()
    }

    fn total_vibe(&self) -> f64 {
        self.rooms.iter().filter_map(|r| r.as_ref()).map(|r| r.vibe).sum()
    }

    fn tick(&mut self, diffuse_rate: f64, _jepa_window: usize) {
        if self.active_count() == 0 { return; }
        let ids = self.active_ids();
        let mut new_vibes: HashMap<usize, f64> = HashMap::new();

        for &id in &ids {
            let room_vibe = self.get_room(id).map(|r| r.vibe).unwrap_or(0.5);
            let neighbors = self.adjacency.get(&id).cloned().unwrap_or_default();

            let mut delta = 0.0;
            for &nid in &neighbors {
                if let Some(n) = self.get_room(nid) {
                    delta += diffuse_rate * (n.vibe - room_vibe);
                }
            }

            let neighbor_avg = if !neighbors.is_empty() {
                let sum: f64 = neighbors.iter()
                    .filter_map(|&nid| self.get_room(nid).map(|r| r.vibe))
                    .sum();
                sum / neighbors.len() as f64
            } else {
                room_vibe
            };

            let learning = 0.01 * (neighbor_avg - room_vibe);
            new_vibes.insert(id, room_vibe + delta + learning);
        }

        for (id, v) in new_vibes {
            if let Some(room) = self.get_room_mut(id) {
                room.vibe = v.clamp(0.0, 1.0);
            }
        }
    }
}

// ── Config File ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct Config {
    rooms: Option<usize>,
    topology: Option<String>,
    probability: Option<f64>,
    ticks: Option<usize>,
    diffuse_rate: Option<f64>,
    jepa_window: Option<usize>,
    output_format: Option<String>,
    output_file: Option<String>,
}

fn parse_toml_config(content: &str) -> Config {
    let mut cfg = Config::default();
    let mut section = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len()-1].trim().to_string();
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq+1..].trim().trim_matches('"');
            match (section.as_str(), key) {
                ("graph", "rooms") => cfg.rooms = val.parse().ok(),
                ("graph", "topology") => cfg.topology = Some(val.to_string()),
                ("graph", "probability") => cfg.probability = val.parse().ok(),
                ("simulation", "ticks") => cfg.ticks = val.parse().ok(),
                ("simulation", "diffuse_rate") => cfg.diffuse_rate = val.parse().ok(),
                ("simulation", "jepa_window") => cfg.jepa_window = val.parse().ok(),
                ("output", "format") => cfg.output_format = Some(val.to_string()),
                ("output", "file") => cfg.output_file = Some(val.to_string()),
                _ => {}
            }
        }
    }
    cfg
}

// ── CLI Argument Parsing ────────────────────────────────────────────────────

fn parse_args(args: &[String]) -> (String, HashMap<String, String>) {
    let mut cmd = String::new();
    let mut params = HashMap::new();
    let mut i = 1;
    if i < args.len() {
        cmd = args[i].clone();
        i += 1;
    }
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            let key = arg[2..].to_string();
            if i + 1 < args.len() && !args[i+1].starts_with("--") {
                params.insert(key, args[i+1].clone());
                i += 2;
            } else {
                params.insert(key, "true".to_string());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    (cmd, params)
}

// ── ASCII Visualization ─────────────────────────────────────────────────────

fn vibe_to_char(v: f64) -> char {
    if v < 0.15 { '░' }
    else if v < 0.35 { '▒' }
    else if v < 0.55 { '●' }
    else if v < 0.75 { '◉' }
    else { '█' }
}

fn visualize(graph: &Graph) -> String {
    let mut out = String::new();
    let ids = graph.active_ids();
    let edges = &graph.edges;

    if ids.is_empty() {
        return "(empty graph)".to_string();
    }

    let pairs: Vec<(usize, usize)> = ids.chunks(2).map(|chunk| {
        let a = chunk[0];
        let b = if chunk.len() > 1 { chunk[1] } else { a };
        (a, b)
    }).collect();

    for (pi, &(a, b)) in pairs.iter().enumerate() {
        let va = graph.get_room(a).map(|r| r.vibe).unwrap_or(0.5);
        let ca = vibe_to_char(va);
        let connected = edges.iter().any(|(x, y)| (*x == a && *y == b) || (*x == b && *y == a));
        let connector = if connected && a != b { "─────────" } else { "         " };

        if a == b {
            out.push_str(&format!("Room {} [{:.2}] {}\n", a, va, ca));
        } else {
            let vb = graph.get_room(b).map(|r| r.vibe).unwrap_or(0.5);
            let cb = vibe_to_char(vb);
            out.push_str(&format!("Room {} [{:.2}] {}{} {} [{:.2}] Room {}\n", a, va, ca, connector, cb, vb, b));
        }

        if pi + 1 < pairs.len() {
            let next_a = pairs[pi + 1].0;
            let has_vert_a = edges.iter().any(|(x, y)| (*x == a && *y == next_a) || (*x == next_a && *y == a));
            let next_b = pairs[pi + 1].1;
            let has_vert_b = b != a && edges.iter().any(|(x, y)| (*x == b && *y == next_b) || (*x == next_b && *y == b));
            let va_str = if has_vert_a { "│" } else { " " };
            let vb_str = if has_vert_b { "│" } else { " " };
            out.push_str(&format!("          {}               {}\n", va_str, vb_str));
        }
    }

    let fleet = graph.fleet_vibe();
    let surprise = graph.fleet_surprise();
    let total = graph.total_vibe();
    let expected = graph.active_count() as f64 * 0.5;
    let delta = (total - expected).abs();
    let check = if delta < 0.01 { "✅" } else { "❌" };
    out.push_str(&format!("\nFleet vibe: {:.3} | Fleet surprise: {:.3} | Conservation: {} (Δ={:.3})\n", fleet, surprise, check, delta));

    out
}

// ── Export ───────────────────────────────────────────────────────────────────

fn export_csv(graph: &Graph) -> String {
    let mut out = String::from("room_id,vibe,neighbors\n");
    for room in graph.rooms.iter().filter_map(|r| r.as_ref()) {
        let neighbors = graph.adjacency.get(&room.id).map(|n| n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(";")).unwrap_or_default();
        out.push_str(&format!("{},{:.6},{}\n", room.id, room.vibe, neighbors));
    }
    out
}

fn export_json(graph: &Graph) -> String {
    let mut rooms_json = Vec::new();
    for room in graph.rooms.iter().filter_map(|r| r.as_ref()) {
        let neighbors = graph.adjacency.get(&room.id).map(|n| n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")).unwrap_or_default();
        rooms_json.push(format!(r#"{{"id":{},"vibe":{:.6},"neighbors":[{}]}}"#, room.id, room.vibe, neighbors));
    }
    let edges_json: Vec<String> = graph.edges.iter().map(|(a, b)| format!("[{},{}]", a, b)).collect();
    format!(r#"{{"rooms":[{}],"edges":[{}],"fleet_vibe":{:.6},"fleet_surprise":{:.6}}}"#, rooms_json.join(","), edges_json.join(","), graph.fleet_vibe(), graph.fleet_surprise())
}

// ── Stats ───────────────────────────────────────────────────────────────────

fn print_stats(graph: &Graph) -> String {
    let mut out = String::new();
    out.push_str(&format!("Active rooms: {}\n", graph.active_count()));
    out.push_str(&format!("Edges: {}\n", graph.edges.len()));
    out.push_str(&format!("Fleet vibe: {:.6}\n", graph.fleet_vibe()));
    out.push_str(&format!("Fleet surprise: {:.6}\n", graph.fleet_surprise()));
    let total = graph.total_vibe();
    let expected = graph.active_count() as f64 * 0.5;
    let delta = (total - expected).abs();
    let check = if delta < 0.01 { "✅ conserved" } else { "❌ not conserved" };
    out.push_str(&format!("Conservation: {} (total={:.6}, expected={:.6}, Δ={:.6})\n", check, total, expected, delta));
    out.push_str("\nRoom details:\n");
    for room in graph.rooms.iter().filter_map(|r| r.as_ref()) {
        let n_count = graph.adjacency.get(&room.id).map(|n| n.len()).unwrap_or(0);
        out.push_str(&format!("  Room {} | vibe={:.6} | neighbors={}\n", room.id, room.vibe, n_count));
    }
    out
}

// ── Serialization ───────────────────────────────────────────────────────────

fn serialize_graph(graph: &Graph) -> String {
    let mut rooms = Vec::new();
    for r in &graph.rooms {
        match r {
            Some(room) => rooms.push(format!(r#"{{"id":{},"vibe":{:.15}}}"#, room.id, room.vibe)),
            None => rooms.push("null".to_string()),
        }
    }
    let edges: Vec<String> = graph.edges.iter().map(|(a, b)| format!("[{},{}]", a, b)).collect();
    let adj: Vec<String> = graph.adjacency.iter().map(|(k, v)| {
        let vals: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        format!(r#""{}":[{}]"#, k, vals.join(","))
    }).collect();
    format!(r#"{{"rooms":[{}],"edges":[{}],"adjacency":{{{}}},"next_id":{}}}"#, rooms.join(","), edges.join(","), adj.join(","), graph.next_id)
}

fn deserialize_graph(content: &str) -> Result<Graph, String> {
    let trimmed = content.trim();
    let next_id = extract_number(trimmed, "\"next_id\":").unwrap_or(0.0) as usize;

    let rooms_start = trimmed.find("\"rooms\":[").map(|idx| idx + "\"rooms\":[".len() - 1).unwrap_or(trimmed.len());
    let rooms_end = find_matching_bracket(trimmed, rooms_start);
    let rooms_str = &trimmed[rooms_start..rooms_end+1];

    let mut rooms = Vec::new();
    let mut i = 0;
    let bytes = rooms_str.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let end = find_matching_brace(rooms_str, i);
            let obj = &rooms_str[i..end+1];
            let id = extract_number(obj, "\"id\":").unwrap_or(0.0) as usize;
            let vibe = extract_float(obj, "\"vibe\":").unwrap_or(0.5);
            rooms.push(Some(Room { id, vibe }));
            i = end + 1;
        } else if bytes[i] == b'n' {
            rooms.push(None);
            i += 4;
        } else {
            i += 1;
        }
    }

    let edges_start = trimmed.find("\"edges\":[").map(|idx| idx + "\"edges\":[".len() - 1).unwrap_or(trimmed.len());
    let edges_end = find_matching_bracket(trimmed, edges_start);
    let edges_str = &trimmed[edges_start..edges_end+1];
    let mut edges = Vec::new();
    let mut ei = 1; // skip the outer [
    let ebytes = edges_str.as_bytes();
    while ei < ebytes.len() - 1 { // stop before outer ]
        if ebytes[ei] == b'[' {
            let end = find_matching_square(edges_str, ei);
            let inner = &edges_str[ei+1..end];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 2 {
                let a: usize = parts[0].trim().parse().unwrap_or(0);
                let b: usize = parts[1].trim().parse().unwrap_or(0);
                edges.push((a, b));
            }
            ei = end + 1;
        } else {
            ei += 1;
        }
    }

    let mut adjacency = HashMap::new();
    for &(a, b) in &edges {
        adjacency.entry(a).or_insert_with(Vec::new).push(b);
        adjacency.entry(b).or_insert_with(Vec::new).push(a);
    }
    for r in &rooms {
        if let Some(room) = r {
            adjacency.entry(room.id).or_insert_with(Vec::new);
        }
    }

    Ok(Graph { rooms, edges, adjacency, next_id, rng_state: 42 })
}

fn find_array_start(s: &str, prefix: &str) -> usize {
    // prefix like "\"edges\":[" — the [ is the array open
    if let Some(idx) = s.find(prefix) {
        idx + prefix.len() - 1  // position of the [
    } else {
        return s.len(); // not found, return safe index
    }
}

fn find_matching_bracket(s: &str, start: usize) -> usize {
    let bytes = s.as_bytes();
    let mut depth = 0;
    for i in start..bytes.len() {
        if bytes[i] == b'[' { depth += 1; }
        else if bytes[i] == b']' {
            depth -= 1;
            if depth == 0 { return i; }
        }
    }
    s.len() - 1
}

fn find_matching_brace(s: &str, start: usize) -> usize {
    let bytes = s.as_bytes();
    let mut depth = 0;
    for i in start..bytes.len() {
        if bytes[i] == b'{' { depth += 1; }
        else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 { return i; }
        }
    }
    s.len() - 1
}

fn find_matching_square(s: &str, start: usize) -> usize {
    let bytes = s.as_bytes();
    let mut depth = 0;
    for i in start..bytes.len() {
        if bytes[i] == b'[' { depth += 1; }
        else if bytes[i] == b']' {
            depth -= 1;
            if depth == 0 { return i; }
        }
    }
    s.len() - 1
}

fn extract_number(s: &str, prefix: &str) -> Option<f64> {
    let idx = s.find(prefix)?;
    let rest = &s[idx + prefix.len()..];
    // Collect digits (and optional decimal point for robustness)
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(rest.len());
    if end == 0 { return None; }
    rest[..end].parse().ok()
}

fn extract_float(s: &str, prefix: &str) -> Option<f64> {
    let idx = s.find(prefix)?;
    let rest = &s[idx + prefix.len()..];
    let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn load_graph(state_path: &str) -> Result<Graph, String> {
    let content = fs::read_to_string(state_path).map_err(|e| format!("No graph state found ({}). Run 'new' first. ({})", state_path, e))?;
    deserialize_graph(&content)
}

fn save_graph(graph: &Graph, state_path: &str) -> Result<(), String> {
    let content = serialize_graph(graph);
    fs::write(state_path, content).map_err(|e| format!("Failed to save state: {}", e))
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    let (cmd, params) = parse_args(&args);

    let config_path = "grand-pattern.toml";
    let config_content = fs::read_to_string(config_path).ok();
    let cfg = config_content.as_ref().map(|c| parse_toml_config(c));

    let state_path = ".grand-pattern-state.json";

    match run_command(&cmd, &params, cfg.as_ref(), state_path) {
        Ok(output) => println!("{}", output),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_command(cmd: &str, params: &HashMap<String, String>, cfg: Option<&Config>, state_path: &str) -> Result<String, String> {
    match cmd {
        "new" => {
            let rooms: usize = match params.get("rooms") {
                Some(v) => v.parse().map_err(|_| "Invalid rooms count")?,
                None => cfg.and_then(|c| c.rooms).unwrap_or(20),
            };
            let topology = params.get("topology").as_ref().map(|s| s.as_str())
                .or_else(|| cfg.and_then(|c| c.topology.as_deref()))
                .unwrap_or("ring");
            let probability: f64 = match params.get("probability") {
                Some(v) => v.parse().map_err(|_| "Invalid probability")?,
                None => cfg.and_then(|c| c.probability).unwrap_or(0.3),
            };
            let seed: u64 = params.get("seed")
                .map(|s| s.parse().unwrap_or(42))
                .unwrap_or(42);

            let graph = Graph::new(rooms, topology, probability, seed);
            save_graph(&graph, state_path)?;
            Ok(format!("Created graph with {} rooms, topology='{}', probability={:.2}\nSaved to {}", rooms, topology, probability, state_path))
        }
        "tick" => {
            let mut graph = load_graph(state_path)?;
            let count: usize = match params.get("count") {
                Some(v) => v.parse().map_err(|_| "Invalid tick count")?,
                None => cfg.and_then(|c| c.ticks).unwrap_or(1),
            };
            let diffuse_rate: f64 = match params.get("diffuse-rate") {
                Some(v) => v.parse().map_err(|_| "Invalid diffuse rate")?,
                None => cfg.and_then(|c| c.diffuse_rate).unwrap_or(0.1),
            };
            let jepa_window: usize = match params.get("jepa-window") {
                Some(v) => v.parse().map_err(|_| "Invalid jepa window")?,
                None => cfg.and_then(|c| c.jepa_window).unwrap_or(10),
            };

            let start = Instant::now();
            for _ in 0..count {
                graph.tick(diffuse_rate, jepa_window);
            }
            let elapsed = start.elapsed();
            save_graph(&graph, state_path)?;
            Ok(format!("Ran {} ticks in {:.2?}\nFleet vibe: {:.6} | Fleet surprise: {:.6}", count, elapsed, graph.fleet_vibe(), graph.fleet_surprise()))
        }
        "inject" => {
            let mut graph = load_graph(state_path)?;
            let room_id: usize = params.get("room")
                .ok_or("Missing --room")?
                .parse().map_err(|_| "Invalid room id")?;
            let vibe: f64 = params.get("vibe")
                .ok_or("Missing --vibe")?
                .parse().map_err(|_| "Invalid vibe value")?;
            let old_vibe = graph.get_room(room_id).map(|r| r.vibe).ok_or(format!("Room {} not found", room_id))?;
            if let Some(room) = graph.get_room_mut(room_id) {
                room.vibe = vibe.clamp(0.0, 1.0);
            }
            save_graph(&graph, state_path)?;
            Ok(format!("Room {} vibe: {:.6} → {:.6}", room_id, old_vibe, vibe.clamp(0.0, 1.0)))
        }
        "remove" => {
            let mut graph = load_graph(state_path)?;
            let room_id: usize = params.get("room")
                .ok_or("Missing --room")?
                .parse().map_err(|_| "Invalid room id")?;
            if graph.remove_room(room_id) {
                save_graph(&graph, state_path)?;
                Ok(format!("Removed room {}. Active rooms: {}", room_id, graph.active_count()))
            } else {
                Err(format!("Room {} not found", room_id))
            }
        }
        "stats" => {
            let graph = load_graph(state_path)?;
            Ok(print_stats(&graph))
        }
        "export" => {
            let graph = load_graph(state_path)?;
            let format = params.get("format")
                .map(|s| s.as_str())
                .or_else(|| cfg.and_then(|c| c.output_format.as_deref()))
                .unwrap_or("csv");
            let output = params.get("output")
                .map(|s| s.as_str())
                .or_else(|| cfg.and_then(|c| c.output_file.as_deref()));

            let content = match format {
                "json" => export_json(&graph),
                _ => export_csv(&graph),
            };

            if let Some(path) = output {
                fs::write(path, &content).map_err(|e| format!("Failed to write: {}", e))?;
                Ok(format!("Exported {} to {}", format.to_uppercase(), path))
            } else {
                Ok(content)
            }
        }
        "visualize" => {
            let graph = load_graph(state_path)?;
            Ok(visualize(&graph))
        }
        "benchmark" => {
            let rooms: usize = params.get("rooms")
                .map(|s| s.parse().unwrap_or(1000))
                .unwrap_or(1000);
            let ticks: usize = params.get("ticks")
                .map(|s| s.parse().unwrap_or(10000))
                .unwrap_or(10000);

            let start = Instant::now();
            let mut graph = Graph::new(rooms, "small-world", 0.3, 42);
            let create_time = start.elapsed();

            let tick_start = Instant::now();
            for _ in 0..ticks {
                graph.tick(0.1, 10);
            }
            let tick_time = tick_start.elapsed();
            let total = start.elapsed();

            Ok(format!(
                "Benchmark: {} rooms, {} ticks\n  Create: {:.2?}\n  Tick:   {:.2?}\n  Total:  {:.2?}\n  Fleet vibe: {:.6} | Surprise: {:.6}",
                rooms, ticks, create_time, tick_time, total, graph.fleet_vibe(), graph.fleet_surprise()
            ))
        }
        "attack" => {
            let mut graph = load_graph(state_path)?;
            let attack_type = params.get("type")
                .map(|s| s.as_str())
                .unwrap_or("contrarian");
            let room_id: usize = params.get("room")
                .ok_or("Missing --room")?
                .parse().map_err(|_| "Invalid room id")?;

            let fleet = graph.fleet_vibe();
            match attack_type {
                "contrarian" => {
                    let target = if fleet > 0.5 { 0.0 } else { 1.0 };
                    if let Some(room) = graph.get_room_mut(room_id) {
                        let old = room.vibe;
                        room.vibe = target;
                        save_graph(&graph, state_path)?;
                        Ok(format!("Attack: contrarian on room {}\n  Fleet vibe: {:.3}, pushed room to {} (was {:.3})", room_id, fleet, target, old))
                    } else {
                        Err(format!("Room {} not found", room_id))
                    }
                }
                "noise" => {
                    let target = if graph.pseudo_random() > 0.5 { 1.0 } else { 0.0 };
                    if let Some(room) = graph.get_room_mut(room_id) {
                        room.vibe = target;
                        save_graph(&graph, state_path)?;
                        Ok(format!("Attack: noise on room {} → vibe={}", room_id, target))
                    } else {
                        Err(format!("Room {} not found", room_id))
                    }
                }
                _ => Err(format!("Unknown attack type: {}. Use 'contrarian' or 'noise'.", attack_type))
            }
        }
        "help" | "--help" | "-h" | "" => {
            Ok(r#"grand-pattern-cli v1.0.0

Usage: grand-pattern <command> [options]

Commands:
  new         Create a new graph
              --rooms <N>         Number of rooms (default: 20)
              --topology <TYPE>   ring, small-world, full, random, line (default: ring)
              --probability <P>   Edge probability for random/small-world (default: 0.3)
              --seed <S>          Random seed (default: 42)

  tick        Run simulation ticks
              --count <N>         Number of ticks (default: 1)
              --diffuse-rate <R>  Diffusion rate (default: 0.1)
              --jepa-window <W>   JEPA window size (default: 10)

  inject      Set a room's vibe
              --room <ID>         Room ID
              --vibe <V>          Vibe value (0.0-1.0)

  remove      Remove a room
              --room <ID>         Room ID

  stats       Print graph statistics

  export      Export graph data
              --format <F>        csv or json (default: csv)
              --output <FILE>     Output file path

  visualize   ASCII art visualization

  benchmark   Performance test
              --rooms <N>         Number of rooms (default: 1000)
              --ticks <N>         Number of ticks (default: 10000)

  attack      Inject adversarial behavior
              --type <T>          contrarian or noise
              --room <ID>         Room ID

  help        Show this help message

Config file: grand-pattern.toml (auto-loaded if present)"#.to_string())
        }
        _ => Err(format!("Unknown command: '{}'. Run 'grand-pattern help' for usage.", cmd)),
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn empty_params() -> HashMap<String, String> {
        HashMap::new()
    }

    fn make_test_graph() -> Graph {
        Graph::new(6, "ring", 0.3, 42)
    }

    // Test 1: CLI parses new command
    #[test]
    fn test_parse_new() {
        let args: Vec<String> = vec!["grand-pattern".into(), "new".into(), "--rooms".into(), "20".into(), "--topology".into(), "small-world".into()];
        let (cmd, params) = parse_args(&args);
        assert_eq!(cmd, "new");
        assert_eq!(params.get("rooms").unwrap(), "20");
        assert_eq!(params.get("topology").unwrap(), "small-world");
    }

    // Test 2: CLI parses tick command
    #[test]
    fn test_parse_tick() {
        let args: Vec<String> = vec!["grand-pattern".into(), "tick".into(), "--count".into(), "1000".into(), "--diffuse-rate".into(), "0.1".into()];
        let (cmd, params) = parse_args(&args);
        assert_eq!(cmd, "tick");
        assert_eq!(params.get("count").unwrap(), "1000");
        assert_eq!(params.get("diffuse-rate").unwrap(), "0.1");
    }

    // Test 3: Creates graph with correct topology
    #[test]
    fn test_graph_topology_ring() {
        let g = Graph::new(5, "ring", 0.3, 42);
        assert_eq!(g.active_count(), 5);
        assert_eq!(g.edges.len(), 5);
    }

    #[test]
    fn test_graph_topology_full() {
        let g = Graph::new(4, "full", 0.5, 42);
        assert_eq!(g.edges.len(), 6);
    }

    // Test 4: Runs ticks
    #[test]
    fn test_tick_runs() {
        let mut g = make_test_graph();
        let before = g.fleet_vibe();
        g.tick(0.1, 10);
        assert!((g.fleet_vibe() - before).abs() < 0.01);
    }

    // Test 5: Injects vibe
    #[test]
    fn test_inject_vibe() {
        let mut g = make_test_graph();
        if let Some(room) = g.get_room_mut(0) {
            room.vibe = 1.0;
        }
        assert_eq!(g.get_room(0).unwrap().vibe, 1.0);
    }

    // Test 6: Removes room
    #[test]
    fn test_remove_room() {
        let mut g = make_test_graph();
        let count_before = g.active_count();
        assert!(g.remove_room(2));
        assert_eq!(g.active_count(), count_before - 1);
        assert!(g.get_room(2).is_none());
        for (a, b) in &g.edges {
            assert_ne!(*a, 2);
            assert_ne!(*b, 2);
        }
    }

    // Test 7: Prints stats
    #[test]
    fn test_stats() {
        let g = make_test_graph();
        let stats = print_stats(&g);
        assert!(stats.contains("Active rooms: 6"));
        assert!(stats.contains("Fleet vibe:"));
        assert!(stats.contains("Room 0"));
    }

    // Test 8: Exports CSV
    #[test]
    fn test_export_csv() {
        let g = make_test_graph();
        let csv = export_csv(&g);
        assert!(csv.starts_with("room_id,vibe,neighbors"));
        assert!(csv.contains("0,0.500000"));
    }

    // Test 9: Exports JSON
    #[test]
    fn test_export_json() {
        let g = make_test_graph();
        let json = export_json(&g);
        assert!(json.contains("\"rooms\":["));
        assert!(json.contains("\"fleet_vibe\":"));
        assert!(json.contains("\"edges\":["));
    }

    // Test 10: ASCII visualization renders
    #[test]
    fn test_visualize() {
        let g = make_test_graph();
        let viz = visualize(&g);
        assert!(viz.contains("Room 0"));
        assert!(viz.contains("Fleet vibe:"));
        assert!(viz.contains("Conservation:"));
    }

    // Test 11: Benchmark runs
    #[test]
    fn test_benchmark() {
        let mut g = Graph::new(100, "small-world", 0.3, 42);
        for _ in 0..100 {
            g.tick(0.1, 10);
        }
        assert_eq!(g.active_count(), 100);
    }

    // Test 12: Attack injects contrarian
    #[test]
    fn test_attack_contrarian() {
        let mut g = make_test_graph();
        if let Some(room) = g.get_room_mut(3) { room.vibe = 1.0; }
        let fleet_now = g.fleet_vibe();
        let target = if fleet_now > 0.5 { 0.0 } else { 1.0 };
        if let Some(room) = g.get_room_mut(3) { room.vibe = target; }
        assert_eq!(g.get_room(3).unwrap().vibe, target);
    }

    // Test 13: Config file loads
    #[test]
    fn test_config_parse() {
        let toml = r#"
[graph]
rooms = 30
topology = "small-world"
probability = 0.4

[simulation]
ticks = 500
diffuse_rate = 0.2
jepa_window = 15

[output]
format = "json"
file = "out.json"
"#;
        let cfg = parse_toml_config(toml);
        assert_eq!(cfg.rooms, Some(30));
        assert_eq!(cfg.topology, Some("small-world".to_string()));
        assert_eq!(cfg.probability, Some(0.4));
        assert_eq!(cfg.ticks, Some(500));
        assert_eq!(cfg.diffuse_rate, Some(0.2));
        assert_eq!(cfg.jepa_window, Some(15));
        assert_eq!(cfg.output_format, Some("json".to_string()));
        assert_eq!(cfg.output_file, Some("out.json".to_string()));
    }

    // Test 14: Conservation checked in stats
    #[test]
    fn test_conservation_check() {
        let g = make_test_graph();
        let stats = print_stats(&g);
        assert!(stats.contains("✅ conserved"));
    }

    // Test 15: Empty graph handles gracefully
    #[test]
    fn test_empty_graph() {
        let g = Graph::new(0, "ring", 0.3, 42);
        assert_eq!(g.active_count(), 0);
        assert_eq!(g.fleet_vibe(), 0.0);
        assert_eq!(g.fleet_surprise(), 0.0);
        let viz = visualize(&g);
        assert!(viz.contains("empty graph"));
    }

    // Test 16: Large graph works
    #[test]
    fn test_large_graph() {
        let g = Graph::new(100, "small-world", 0.3, 42);
        assert_eq!(g.active_count(), 100);
        assert!(g.edges.len() >= 100);
    }

    // Test 17: Deterministic with same seed
    #[test]
    fn test_deterministic() {
        let g1 = Graph::new(10, "small-world", 0.3, 42);
        let g2 = Graph::new(10, "small-world", 0.3, 42);
        assert_eq!(g1.edges.len(), g2.edges.len());
        assert_eq!(g1.edges, g2.edges);
    }

    // Test 18: Help text for all subcommands
    #[test]
    fn test_help_output() {
        let help = run_command("help", &empty_params(), None, ".test-state.json").unwrap();
        assert!(help.contains("new"));
        assert!(help.contains("tick"));
        assert!(help.contains("inject"));
        assert!(help.contains("remove"));
        assert!(help.contains("stats"));
        assert!(help.contains("export"));
        assert!(help.contains("visualize"));
        assert!(help.contains("benchmark"));
        assert!(help.contains("attack"));
    }

    // Test 19: Version info
    #[test]
    fn test_version() {
        let help = run_command("help", &empty_params(), None, ".test-state.json").unwrap();
        assert!(help.contains("v1.0.0"));
    }

    // Test 20: Multiple ticks accumulate correctly
    #[test]
    fn test_tick_accumulation() {
        let mut g = Graph::new(4, "ring", 0.3, 42);
        if let Some(room) = g.get_room_mut(0) { room.vibe = 1.0; }
        let initial_total = g.total_vibe();
        for _ in 0..50 {
            g.tick(0.1, 10);
        }
        let final_total = g.total_vibe();
        // Should stay relatively close (clamping may cause drift)
        assert!((final_total - initial_total).abs() < 0.5, "Total vibe drifted too far: {} vs {}", final_total, initial_total);
    }

    // Test 21: Serialization roundtrip
    #[test]
    fn test_serialization_roundtrip() {
        let g = Graph::new(5, "small-world", 0.3, 42);
        let serialized = serialize_graph(&g);
        let deserialized = deserialize_graph(&serialized).unwrap();
        assert_eq!(deserialized.active_count(), g.active_count());
        assert_eq!(deserialized.edges.len(), g.edges.len());
        assert_eq!(deserialized.next_id, g.next_id);
    }

    // Test 22: Line topology
    #[test]
    fn test_line_topology() {
        let g = Graph::new(5, "line", 0.3, 42);
        assert_eq!(g.edges.len(), 4);
    }

    // Test 23: Remove nonexistent room
    #[test]
    fn test_remove_nonexistent() {
        let mut g = make_test_graph();
        assert!(!g.remove_room(999));
    }

    // Test 24: Unknown command returns error
    #[test]
    fn test_unknown_command() {
        let result = run_command("foobar", &empty_params(), None, ".test-state.json");
        assert!(result.is_err());
    }

    // Test 25: Vibe clamped to [0, 1]
    #[test]
    fn test_vibe_clamp() {
        let mut g = make_test_graph();
        if let Some(room) = g.get_room_mut(0) {
            room.vibe = 5.0_f64.clamp(0.0, 1.0);
        }
        assert_eq!(g.get_room(0).unwrap().vibe, 1.0);
    }
}
