mod config;
mod optimizer;
mod report;
mod scanner;

use clap::{Parser, Subcommand};
use colored::*;
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

/// 🦀 ThinFlux - Otimizador de PNG de alta performance escrito em Rust.
/// Desenvolvido para processamento em massa com segurança e velocidade.#[derive(Parser)]
#[derive(Parser)]
#[command(
    author = "Roberto German Guedes Neto",
    version = "0.1.2",
    name = "ThinFlux",
    about,
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 🚀 Inicia a otimização dos arquivos no diretório especificado
    Run {
        /// Caminho para o diretório contendo as imagens PNG
        #[arg(value_name = "DIRETÓRIO")]
        path: PathBuf,

        /// Exibe um resumo visual detalhado no terminal ao finalizar
        #[arg(short, long, default_value_t = false)]
        summary: bool,
    },
    /// ⚙️ Inicializa o arquivo de configuração padrão (thinflux.toml)
    Init,
}

fn main() {
    let cli = Cli::parse();
    let settings = config::load_config();

    match cli.command {
        Commands::Run { path, summary } => {
            println!(
                "🛠️ Otimizando em nível: {}",
                settings.level.to_string().green()
            );
            println!("{}", format!("🔍 Varrendo diretório: {:?}", path).blue());

            let start_time = Instant::now();

            // 1. Scanner
            let files = scanner::find_png_files(path);
            if files.is_empty() {
                println!("{}", "⚠️ Nenhum arquivo PNG encontrado".yellow());
                return;
            }

            println!(
                "📦 Encontrados: {} arquivos.",
                files.len().to_string().green()
            );

            let pb = ProgressBar::new(files.len() as u64);

            // 2. Optimizer + Rayon
            let results: Vec<_> = files
                .par_iter()
                .map(|file| {
                    let res = optimizer::optimize_png(file);
                    pb.inc(1);
                    res
                })
                .collect();

            pb.finish_and_clear();

            // 3. Preparação das métricas para o Relatório
            let report_data: Vec<(PathBuf, usize, usize)> = files
                .into_iter()
                .zip(results)
                .filter_map(|(path, res)| match res {
                    Ok((orig, optim)) => Some((path, orig, optim)),
                    Err(e) => {
                        eprintln!("{}", e);
                        None
                    }
                })
                .collect();

            // 4. Geração do Relatório
            match report::generate_json_report(report_data) {
                Ok(full_report) => {
                    if summary {
                        report::print_terminal_summary(&full_report);
                    }
                    let duration = start_time.elapsed();
                    println!("✅ Concluído em {:?}!", duration);
                    println!("📝 Relatório detalhado gerado em 'thinflux_report.json'");
                }
                Err(e) => eprintln!("Erro ao gerar relatório: {}", e),
            }
        }

        Commands::Init => {
            println!("{}", "🛠️ Gerando arquivo de configuração...{}".blue());
            match config::create_default_config() {
                Ok(_) => println!("✅ Arquivo 'thinflux.toml criado com sucesso!"),
                Err(e) => eprintln!("Erro: {}", e),
            }
        }
    }
}
