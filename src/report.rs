use chrono::{DateTime, Utc};
use colored::*;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct FileMetrics {
    pub name: String,
    pub path: String,
    pub original_kb: f64,
    pub optimized_kb: f64,
    pub saved_kb: f64,
    pub ratio: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub files_processed: usize,
    pub total_original_size_kb: f64,
    pub total_optimized_size_kb: f64,
    pub space_saved_kb: f64,
    pub efficiency_gain_percent: f64,
}

#[derive(Serialize)]
pub struct FinalReport {
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub summary: Summary,
    pub files: Vec<FileMetrics>,
}

pub fn generate_json_report(
    raw_metrics: Vec<(PathBuf, usize, usize)>,
) -> Result<FinalReport, String> {
    let mut files = Vec::new();
    let mut total_orig_bytes = 0;
    let mut total_optim_bytes = 0;

    for (path, orig, optim) in raw_metrics {
        total_orig_bytes += orig;
        total_optim_bytes += optim;

        let saved = orig.saturating_sub(optim);
        let ratio_val = if orig > 0 {
            (saved as f64 / orig as f64) * 100.0
        } else {
            0.0
        };

        files.push(FileMetrics {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            path: path.to_string_lossy().into(),
            original_kb: orig as f64 / 1024.0,
            optimized_kb: optim as f64 / 1024.0,
            saved_kb: saved as f64 / 1024.0,
            ratio: format!("{:.1}%", ratio_val),
        });
    }

    let total_saved = total_orig_bytes.saturating_sub(total_optim_bytes);
    let efficiency = if total_orig_bytes > 0 {
        (total_saved as f64 / total_orig_bytes as f64) * 100.0
    } else {
        0.0
    };

    let report = FinalReport {
        timestamp: Utc::now(),
        status: "success".to_string(),
        summary: Summary {
            files_processed: files.len(),
            total_original_size_kb: total_orig_bytes as f64 / 1024.0,
            total_optimized_size_kb: total_optim_bytes as f64 / 1024.0,
            space_saved_kb: total_saved as f64 / 1024.0,
            efficiency_gain_percent: efficiency,
        },
        files,
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;

    let mut file = File::create("thinflux_report.json")
        .map_err(|e| format!("Erro ao criar arquivo: {}", e))?;

    // CORREÇÃO: Propaga o erro corretamente usando o operador `?` em vez de mascarar com `let _`
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Erro ao gravar arquivo: {}", e))?;

    Ok(report)
}

pub fn print_terminal_summary(report: &FinalReport) {
    let separator = "----------------------------------------------------------------------".bold();

    println!("\n{}", separator);
    // AJUSTE: Atualizado o nome do projeto de OPTIRUST para THINFLUX
    println!("{}", "✨ THINFLUX - RELATÓRIO DE OTIMIZAÇÃO ✨".bold());
    println!("{}", separator);

    println!(
        "{:<10} {:<25} {:>11} {:>11} {:>8}",
        "STATUS".bold(),
        "ARQUIVO".bold(),
        "ORIGINAL".bold(),
        "OTIMIZADO".bold(),
        "GANHO".bold()
    );
    println!("{}", "-".repeat(70));

    for file in &report.files {
        let status = if file.saved_kb > 0.0 {
            "[SUCCESS]".green()
        } else {
            "[SKIPPED]".yellow()
        };

        println!(
            "{:<10} {:<25} {:>8.1} KB {:>8.1} KB {:>10}",
            status,
            file.name.chars().take(22).collect::<String>(),
            file.original_kb,
            file.optimized_kb,
            format!("[ {} ]", file.ratio).green()
        );
    }

    println!("{}", "-".repeat(70));
    println!("\n{} RESUMO DA OPERAÇÃO:", "📊".bold());
    println!(
        "🔹 Arquivos processados: {}",
        report.summary.files_processed.to_string().blue()
    );
    println!(
        "{}",
        format!(
            "🔹 Economia total:      {:.2} KB",
            report.summary.space_saved_kb
        )
        .blue()
    );
    println!(
        "{}",
        format!(
            "🔹 Ganho de eficiência: {:.1}%\n",
            report.summary.efficiency_gain_percent
        )
        .green()
        .bold()
    );
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_kb_conversion_logic() {
        let bytes: usize = 1024;
        let kb = bytes as f64 / 1024.0;
        assert_eq!(kb, 1.0);
    }
}
