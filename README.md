## asker-rs

Терминальный тренажер для подготовки к экзаменам с вопросами и ответами в едином YAML-банке.

### Запуск

```bash
cargo run -- study
```

### Команды

```bash
cargo run -- study --bank exam_bank.yaml --deck portfolio-theory
cargo run -- list
cargo run -- convert --input legacy_questions.txt --output exam_bank.yaml
cargo run -- build --profile release
```

### Сборка бинарного файла

```bash
cargo build --release
./target/release/asker-rs study
```

### Логика работы

Поток тренировки совпадает с предыдущим приложением:

- `1` показать ответ
- `2` знаю
- `3` не знаю
- `q` выйти
