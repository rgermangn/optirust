use std::path::PathBuf;
use walkdir::WalkDir;

/// Varre um diretório recursivamente em busca de arquivos suportados
pub fn find_supported_files(root: PathBuf, custom_types: Option<Vec<String>>) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    let e = ext.to_lowercase();

                    match &custom_types {
                        Some(types) => types.contains(&e),
                        None => {
                            e == "png" || e == "jpg" || e == "jpeg" || e == "webp" || e == "pdf"
                        }
                    }
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_find_supported_files_recursively() {
        // Cria um diretório temporário
        let dir = tempdir().unwrap();
        let sub_dir = dir.path().join("sub");
        fs::create_dir(&sub_dir).unwrap();

        // Cria arquivos de teste com os múltiplos formatos suportados
        File::create(dir.path().join("foto1.png")).unwrap();
        File::create(sub_dir.join("foto2.jpg")).unwrap();
        File::create(dir.path().join("imagem.webp")).unwrap();
        File::create(dir.path().join("doc.pdf")).unwrap();
        File::create(dir.path().join("texto.txt")).unwrap(); // Deve ser ignorado!

        // Chama a nova função de busca atualizada
        let found_files = find_supported_files(dir.path().to_path_buf(), None);

        // Deve encontrar exatamente 4 arquivos válidos (foto1.png, foto2.jpg, imagem.webp e doc.pdf)
        assert_eq!(found_files.len(), 4);

        // Garante que o arquivo .txt não entrou na lista
        let has_txt = found_files
            .iter()
            .any(|p| p.extension().is_some_and(|ext| ext == "txt"));
        assert!(!has_txt, "O scanner não deveria listar arquivos .txt");
    }

    #[test]
    fn test_find_supported_files_with_custom_types() {
        let dir = tempdir().unwrap();

        // Criamos mídias variadas no diretório temporário
        File::create(dir.path().join("foto.png")).unwrap();
        File::create(dir.path().join("imagem.jpg")).unwrap();
        File::create(dir.path().join("doc.pdf")).unwrap();

        // FASE RED: Isso não vai compilar porque a função ainda não aceita o filtro de tipos!
        // Queremos filtrar apenas por png e jpg, ignorando o pdf
        let custom_types = Some(vec!["png".to_string(), "jpg".to_string()]);
        let found_files = find_supported_files(dir.path().to_path_buf(), custom_types);

        assert_eq!(found_files.len(), 2);

        let has_pdf = found_files
            .iter()
            .any(|p| p.extension().is_some_and(|ext| ext == "pdf"));
        assert!(
            !has_pdf,
            "O PDF deveria ter sido filtrado e ignorado pelo escopo customizado!"
        );
    }
}
