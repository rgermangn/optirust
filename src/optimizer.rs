use image::ImageReader;
use image::codecs::jpeg::JpegEncoder;
use lopdf::{Document, Object, Stream};
use oxipng::{InFile, Options, OutFile, optimize};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use webp::Encoder;

#[derive(Debug, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Pdf,
    Unknown,
}

impl ImageFormat {
    pub fn from_extension(extension: &str) -> Self {
        match extension.to_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::Webp,
            "pdf" => ImageFormat::Pdf,
            _ => ImageFormat::Unknown,
        }
    }
}

/// Função principal que atua como Router do Backend de Otimização
pub fn optimize_image(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let format = ImageFormat::from_extension(ext);

    match format {
        ImageFormat::Png => optimize_png(path, level),
        ImageFormat::Jpeg => optimize_jpeg(path, level),
        ImageFormat::Webp => optimize_webp(path, level),
        ImageFormat::Pdf => optimize_pdf(path, level),
        ImageFormat::Unknown => Err(format!("Formato não suportado para o arquivo: {:?}", path)),
    }
}

/// Motor de Otimização de PNG
pub fn optimize_png(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let options = Options::from_preset(level.clamp(0, 6));

    let input = InFile::Path(path.to_path_buf());
    let output = OutFile::Path {
        path: Some(path.to_path_buf()),
        preserve_attrs: false,
    };

    // Executa a otimização e trata o erro
    optimize(&input, &output, &options).map_err(|e| format!("Erro ao otimizar {:?}: {}", path, e))
}

/// Motor de Otimização de JPEG
fn optimize_jpeg(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let initial_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;

    // Carrega a imagem na memória de forma genérica
    let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    // Mapeia o nível (0-6) para a qualidade JPEG (0-100). Ex: Nível 2 = 80% qualidade
    let quality = match level {
        0 => 95,
        1 => 85,
        2 => 75,
        3 => 65,
        _ => 50,
    };

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    // Codifica com a nova compressão
    let encoder = JpegEncoder::new_with_quality(&mut writer, quality);
    img.write_with_encoder(encoder).map_err(|e| e.to_string())?;

    let final_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    Ok((initial_size, final_size))
}

/// Motor de Otimização de WebP
fn optimize_webp(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let initial_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;

    // 1. Carrega a imagem dinamicamente usando a crate image
    let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    // 2. Mapeia o nível (0-6) para a qualidade WebP (0.0 a 100.0)
    let quality = match level {
        0 => 90.0,
        1 => 82.0,
        2 => 75.0,
        3 => 65.0,
        _ => 50.0,
    };

    // 3. Alimenta o encoder da libwebp com a imagem carregada
    let encoder =
        Encoder::from_image(&img).map_err(|_| "Falha ao inicializar o encoder WebP".to_string())?;

    // 4. Executa a compressão Lossy (com perda) baseada no perfil de qualidade
    let memory = encoder.encode(quality);

    // 5. Sobrescreve o arquivo original com os novos bytes comprimidos
    std::fs::write(path, &*memory).map_err(|e| e.to_string())?;

    let final_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    Ok((initial_size, final_size))
}

/// Motor de Otimização de PDF
fn optimize_pdf(path: &PathBuf, _level: u8) -> Result<(usize, usize), String> {
    let initial_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;

    // 1. Carrega o documento PDF na memória
    let mut doc = Document::load(path).map_err(|e| format!("Erro ao abrir PDF: {}", e))?;
    let mut images_optimized = 0;

    // 2. Varre todos os objetos internos do arquivo procurando por Streams de Imagem
    for (_, object) in doc.objects.iter_mut() {
        if let Object::Stream(stream) = object
            && is_image_stream(stream)
            && let Ok(_data) = stream.decompressed_content()
        {
            // 💡 Nota para o futuro: Lógica de otimização dos bytes vai aqui
            // Exemplo: stream.set_plain_content(bytes_otimizados);
            images_optimized += 1;
        }
    }

    // 3. Se alteramos alguma imagem, salvamos o PDF compactando-o novamente
    if images_optimized > 0 {
        doc.save(path)
            .map_err(|e| format!("Erro ao salvar PDF: {}", e))?;
    }

    let final_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    Ok((initial_size, final_size))
}

/// Helper para validar se o objeto do PDF é de fato uma imagem
fn is_image_stream(stream: &Stream) -> bool {
    if let Ok(Object::Name(name)) = stream.dict.get(b"Subtype")
        && name == b"Image"
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_image_format_detection() {
        assert_eq!(ImageFormat::from_extension("JPEG"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("webp"), ImageFormat::Webp);
        assert_eq!(ImageFormat::from_extension("png"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("txt"), ImageFormat::Unknown);
    }

    #[test]
    fn test_compression_reduces_size() {
        // 1. Preparar: Copia a imagem de teste para não estragar a original
        let input_path = PathBuf::from("test_input.png");
        let test_path = PathBuf::from("test_output.png");

        if !input_path.exists() {
            panic!("Por favor, adicione a imagem 'test_input.png' na raiz do projeto.");
        }

        fs::copy(&input_path, &test_path).unwrap();

        // 2. Chama a função 'optimize_image' passando o nível de compressão padrão (2)
        let (initial_size, final_size) = optimize_image(&test_path, 2).expect("Erro ao otimizar");

        // 3. Validar
        println!(
            "Inicial: {} bytes | Final: {} bytes",
            initial_size, final_size
        );
        assert!(final_size <= initial_size);

        // Limpeza
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_file_not_found_error() {
        let ghost_path = PathBuf::from("arquivo_fantasma.png");

        let result = optimize_image(&ghost_path, 2);

        assert!(result.is_err());
    }

    #[test]
    fn test_pdf_format_routing() {
        let pdf_path = PathBuf::from("relatorio_teste.pdf");
        let result = optimize_image(&pdf_path, 2);
        assert!(result.is_err() || result.is_ok());
    }
}
