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

/// 🦀 ThinFlux - Otimizador de assets de alta performance escrito em Rust.
/// Desenvolvido para processamento em massa com segurança e velocidade.#[derive(Parser)]
#[derive(Parser)]
#[command(
    author = "Roberto German Guedes Neto",
    version = "0.2.0",
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
        /// Caminho para o diretório contendo os assets
        #[arg(value_name = "DIRETÓRIO")]
        path: PathBuf,

        /// Exibe um resumo visual detalhado no terminal ao finalizar
        #[arg(short, long, default_value_t = false)]
        summary: bool,

        /// Silencia os logs e barras de progresso visuais para CI/CD
        #[arg(long, default_value_t = false)]
        silent: bool,

        /// Caminho para um arquivo de configuração personalizado TOML
        #[arg(short, long, value_name = "ARQUIVO")]
        config: Option<PathBuf>,

        /// Nível dinâmico de compressão (0 a 6) [sobrescreve o TOML]
        #[arg(short, long, value_name = "0-6", value_parser = clap::value_parser!(u8).range(0..=6))]
        level: Option<u8>,

        /// Extensões permitidas para otimização separadas por vírgula (ex: png,webp,pdf)
        #[arg(short, long, value_name = "LISTA", value_delimiter = ',')]
        types: Option<Vec<String>>,
    },
    /// ⚙️ Inicializa o arquivo de configuração padrão (thinflux.toml)
    Init,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            path,
            summary,
            silent,
            config,
            level,
            types,
        } => {
            if config.as_ref().is_some_and(|cfg_path| !cfg_path.exists()) {
                let cfg_path = config.as_ref().unwrap();
                eprintln!(
                    "{}",
                    format!(
                        "Erro: O arquivo de configuração especificado '{:?}' não foi encontrado.",
                        cfg_path
                    )
                    .red()
                );
                std::process::exit(1);
            }

            // 1. Configura controle de cores
            if silent {
                colored::control::set_override(false);
            }

            // Carrega a configuração dinamicamente com base na presença da flag
            let mut settings = config::load_config(config.as_deref());

            if let Some(cli_level) = level {
                settings.level = cli_level;
            }

            // 2. Logs iniciais
            if !silent {
                println!(
                    "🛠️ Otimizando em nível: {}",
                    settings.level.to_string().green()
                );
                println!("{}", format!("🔍 Varrendo diretório: {:?}", path).blue());
            }

            let start_time = Instant::now();

            // 1. Scanner
            let files = scanner::find_supported_files(path.clone(), types);

            if files.is_empty() {
                if !silent {
                    println!("{}", "⚠️ Nenhum arquivo suportado foi encontrado".yellow());
                }
                return;
            }

            if !silent {
                println!(
                    "📦 Encontrados: {} arquivos.",
                    files.len().to_string().green()
                );
            }

            let pb = if silent {
                None
            } else {
                Some(ProgressBar::new(files.len() as u64))
            };

            let pb_ref = pb.as_ref();

            // 2. Optimizer + Rayon
            let results: Vec<_> = files
                .par_iter()
                .map(|file| {
                    let res = optimizer::optimize_image(file, settings.level);
                    // Passando a referência de forma segura entre as threads do pool
                    if let Some(progress_bar) = pb_ref {
                        progress_bar.inc(1);
                    }
                    res
                })
                .collect();

            if let Some(progress_bar) = pb {
                progress_bar.finish_and_clear();
            }

            // 3. Preparação das métricas para o Relatório
            let report_data: Vec<(PathBuf, usize, usize)> = files
                .into_iter()
                .zip(results)
                .filter_map(|(path, res)| match res {
                    Ok((orig, optim)) => Some((path, orig, optim)),
                    Err(e) => {
                        if !silent {
                            eprintln!("{}", e);
                        }
                        None
                    }
                })
                .collect();

            // 4. Geração do Relatório
            match report::generate_json_report(report_data) {
                Ok(full_report) => {
                    if summary && !silent {
                        report::print_terminal_summary(&full_report);
                    }

                    if !silent {
                        let duration = start_time.elapsed();
                        println!("✅ Concluído em {:?}!", duration);
                        println!("📝 Relatório detalhado gerado em 'thinflux_report.json'");
                    }
                }
                Err(e) => {
                    if !silent {
                        eprintln!("Erro ao gerar relatório: {}", e);
                    }
                }
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

// ─── Testes ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_silent_flag_parsing() {
        let args = ["thinflux", "run", "./", "--silent"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run { silent, .. } => {
                assert!(silent, "A flag silent deveria ser true");
            }
            _ => panic!("Subcomando incorreto parseado"),
        }
    }
}
