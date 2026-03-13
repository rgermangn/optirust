fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        path::PathBuf,
    };
    use tempfile::{TempDir, tempdir};

    #[test]
    fn test_find_png_files_recursively() {
        // Cria um diretório temporário
        let dir: TempDir = tempdir().unwrap();
        let sub_dir: PathBuf = dir.path().join("sub");
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
