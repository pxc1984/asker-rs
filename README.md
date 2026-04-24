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
```

### Формат записи вопросов

Схема находится в файле `question-bank.schema.yaml`, можно добавить в свою IDE и пользоваться подсказками
