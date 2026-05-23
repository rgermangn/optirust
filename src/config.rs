use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct OptiConfig {
    pub level: u8,
    pub overwrite: bool,
}

impl Default for OptiConfig {
    fn default() -> Self {
        Self {
            level: 2,
            overwrite: true,
        }
    }
}

pub fn load_config(custom_path: Option<&Path>) -> OptiConfig {
    let config_path = match custom_path {
        Some(path) => path,
        None => Path::new("thinflux.toml"),
    };

    if config_path.exists() {
        let content = fs::read_to_string(config_path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        OptiConfig::default()
    }
}

pub fn create_default_config() -> Result<(), String> {
    let config_path = Path::new("thinflux.toml");

    if config_path.exists() {
        return Err("O arquivo thinflux.toml já existe neste diretório.".to_string());
    }

    let default_config = OptiConfig::default();

    let toml_content = toml::to_string_pretty(&default_config)
        .map_err(|e| format!("Erro ao gerar TOML: {}", e))?;

    fs::write(config_path, toml_content)
        .map_err(|e| format!("Erro ao criar o arquivo thinflux.toml: {}", e))
}

// ─── Testes ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Sem arquivo de configuração, o padrão é 2
    fn test_default_config_loading() {
        let config = load_config(None);
        assert_eq!(config.level, 2);
    }

    #[test]
    // Testa a leitura de um arquivo de configuração
    fn test_toml_parsing() {
        let toml_content = "level = 5\noverwrite = false";
        let config: OptiConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.level, 5);
        assert!(!config.overwrite);
    }

    #[test]
    fn test_create_default_config() {
        let test_file = "thinflux.toml";
        // Limpeza inicial
        let _ = fs::remove_file(test_file);

        let result = create_default_config();
        assert!(result.is_ok());
        assert!(Path::new(test_file).exists());

        // Valida o conteúdo (level 2 padrão)
        let content = fs::read_to_string(test_file).unwrap();
        assert!(content.contains("level = 2"));

        // Limpeza final
        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_load_custom_config_path() {
        let custom_file = "custom_config.toml";
        // 1. Criamos um arquivo temporário com nível de compressão diferente (ex: 7)
        fs::write(custom_file, "level = 7\noverwrite = false").unwrap();

        // 2. FASE RED: Isso não vai compilar porque load_config() ainda não aceita argumentos!
        // Queremos passar Some(Path::new(custom_file)) para carregar o arquivo customizado
        let config = load_config(Some(Path::new(custom_file)));

        assert_eq!(config.level, 7);
        assert!(!config.overwrite);

        // Limpeza
        fs::remove_file(custom_file).unwrap();
    }
}
