use oxipng::{InFile, Options, OutFile, optimize};
use std::path::PathBuf;

/// Otimiza uma imagem PNG individual
pub fn optimize_png(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let options = Options::from_preset(level.clamp(0, 6));

    let input = InFile::Path(path.to_path_buf());
    let output = OutFile::Path {
        path: Some(path.to_path_buf()),
        preserve_attrs: false,
    };

    // Executa a otimização e trata o erro de forma idiomática
    optimize(&input, &output, &options).map_err(|e| format!("Erro ao otimizar {:?}: {}", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_compression_reduces_size() {
        // 1. Preparar: Copia a imagem de teste para não estragar a original
        let input_path = PathBuf::from("test_input.png");
        let test_path = PathBuf::from("test_output.png");

        // 2. Verifica se a imagem de teste existe antes de começar
        if !input_path.exists() {
            panic!("Por favor, adicione a imagem 'test_input.png' na raiz do projeto.");
        }

        fs::copy(&input_path, &test_path).unwrap();

        // 3. Executar a compressão e campturear os tamanhos retornados
        let (initial_size, final_size) = optimize_png(&test_path, 2).expect("Erro ao otimizar");

        // 4. Validar: O tamanho final deve ser menor ou igual ao inicial
        println!(
            "Inicial: {} bytes | Final: {} bytes",
            initial_size, final_size
        );
        assert!(final_size < initial_size);

        // Limpeza: Remove a imagem de teste
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_file_not_found_error() {
        // 1. Arquivo inexistente
        let ghost_path = PathBuf::from("arquivo_fantasma.png");

        // 2. Executar a otimização e verificar o erro
        let result = optimize_png(&ghost_path, 2);

        // 3. Verificar o erro
        assert!(result.is_err());

        // 4. Checar a mensagem de erro
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Erro ao otimizar"));
    }
}
