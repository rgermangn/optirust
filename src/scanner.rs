use std::path::PathBuf;
use walkdir::WalkDir;

/// Varre um diretório recursivamente em busca de arquivos .png
pub fn find_png_files(root: PathBuf) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok()) // Ignora erros de leitura (ex: pastas sem permissão)
        .filter(|e| e.file_type().is_file()) // Garante que é um arquivo e não uma pasta
        .map(|e| e.path().to_path_buf())
        .filter(|path| {
            path.extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect() // Transforma o iterador em um Vector
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_find_png_files_recursively() {
        // Cria um diretório temporário
        let dir = tempdir().unwrap();
        let sub_dir = dir.path().join("sub");
        fs::create_dir(&sub_dir).unwrap();

        // Cria arquivos de teste
        File::create(dir.path().join("foto1.png")).unwrap();
        File::create(sub_dir.join("foto2.png")).unwrap();
        File::create(dir.path().join("texto.txt")).unwrap();

        // Chama a função de busca
        let found_files = find_png_files(dir.path().to_path_buf());

        // Validação
        assert_eq!(found_files.len(), 2);
    }
}
