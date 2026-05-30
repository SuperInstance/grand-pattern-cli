# grand-pattern-cli

Command-line tool for the Grand Pattern — create, run, visualize, and analyze cellular graph intelligence.

## Install

```bash
cargo install --git https://github.com/SuperInstance/grand-pattern-cli.git
```

Or build from source:

```bash
git clone https://github.com/SuperInstance/grand-pattern-cli.git
cd grand-pattern-cli
cargo build --release
```

## Usage

```bash
# Create a graph
grand-pattern new --rooms 20 --topology small-world --probability 0.3

# Run simulation
grand-pattern tick --count 1000 --diffuse-rate 0.1

# Inject a vibe into a room
grand-pattern inject --room 5 --vibe 1.0

# Remove a room
grand-pattern remove --room 10

# View statistics
grand-pattern stats

# Export data
grand-pattern export --format csv --output data.csv
grand-pattern export --format json --output data.json

# ASCII visualization
grand-pattern visualize

# Performance benchmark
grand-pattern benchmark --rooms 1000 --ticks 10000

# Adversarial attack
grand-pattern attack --type contrarian --room 3
```

## ASCII Visualization

```
Room 0 [0.54] ●─────────● [0.48] Room 1
          │               │
Room 2 [0.51] ●─────────● [0.49] Room 3
          │               │
Room 4 [0.52] ●─────────● [0.50] Room 5

Fleet vibe: 0.507 | Fleet surprise: 0.012 | Conservation: ✅ (Δ=0.000)
```

## Config File

Create `grand-pattern.toml` in your working directory:

```toml
[graph]
rooms = 20
topology = "small-world"
probability = 0.3

[simulation]
ticks = 1000
diffuse_rate = 0.1
jepa_window = 10

[output]
format = "csv"
file = "output.csv"
```

## Topologies

- `ring` — Circular ring (default)
- `small-world` — Ring with random shortcuts
- `full` — Complete graph (every room connected to every other)
- `random` — Erdős–Rényi random graph
- `line` — Linear chain

## Pure Rust, Zero Dependencies

No external crates. Argument parsing, TOML config, JSON serialization — all built from scratch.

## License

MIT
