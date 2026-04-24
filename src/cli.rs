use crate::DEFAULT_BANK_PATH;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
#[command(author, version, about = "Единый тренажер для подготовки к экзаменам")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Запустить тренировку по вопросам")]
    Study {
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        bank: PathBuf,
        #[arg(short, long)]
        deck: Option<String>,
    },
    #[command(about = "Показать доступные колоды")]
    List {
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        bank: PathBuf,
    },
    #[command(about = "Конвертировать старый формат вопросов в YAML")]
    Convert {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        output: PathBuf,
        #[arg(short, long, default_value = "Подготовка к защите проекта")]
        title: String,
        #[arg(long, default_value = "obshchee")]
        deck_id: String,
        #[arg(long, default_value = "Общая")]
        deck_name: String,
    },
}
