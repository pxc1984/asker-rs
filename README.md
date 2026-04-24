## asker-rs

Terminal trainer for exam preparation with questions and answers stored in a single YAML bank.

### Run

```bash
cargo run -- study
```

### Commands

```bash
cargo run -- study --bank exam_bank.yaml --deck portfolio-theory
cargo run -- list
cargo run -- convert --input legacy_questions.txt --output exam_bank.yaml
cargo run -- build --profile release
```

### Build binary

```bash
cargo build --release
./target/release/asker-rs study
```

### Flow

The training flow matches the previous app:

- `1` show answer
- `2` I know
- `3` I don't know
- `q` quit
