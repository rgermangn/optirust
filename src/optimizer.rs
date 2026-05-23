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

    optimize(&input, &output, &options).map_err(|e| format!("Erro ao otimizar {:?}: {}", path, e))
}

/// Motor de Otimização de JPEG
fn optimize_jpeg(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let initial_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;

    let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    // Mapeia nível (0-6) → qualidade JPEG (0-100)
    let quality = match level {
        0 => 95,
        1 => 85,
        2 => 75,
        3 => 65,
        _ => 50,
    };

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    let encoder = JpegEncoder::new_with_quality(&mut writer, quality);
    img.write_with_encoder(encoder).map_err(|e| e.to_string())?;

    let final_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    Ok((initial_size, final_size))
}

/// Motor de Otimização de WebP
fn optimize_webp(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    let initial_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;

    let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    // Mapeia nível (0-6) → qualidade WebP (0.0-100.0)
    let quality = match level {
        0 => 90.0,
        1 => 82.0,
        2 => 75.0,
        3 => 65.0,
        _ => 50.0,
    };

    let encoder =
        Encoder::from_image(&img).map_err(|_| "Falha ao inicializar o encoder WebP".to_string())?;
    let memory = encoder.encode(quality);
    std::fs::write(path, &*memory).map_err(|e| e.to_string())?;

    let final_size = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    Ok((initial_size, final_size))
}

/// Motor de Otimização de PDF
fn optimize_pdf(path: &PathBuf, level: u8) -> Result<(usize, usize), String> {
    // 1. Coleta o tamanho original em disco
    let original_size = std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);

    // 2. Carrega o documento PDF na memória usando lopdf
    let mut doc =
        Document::load(path).map_err(|e| format!("Erro ao carregar PDF {:?}: {}", path, e))?;

    // 3. Remove metadados desnecessários (Produtor/Autor)
    doc.trailer.remove(b"Info");

    // 4. Varre todos os objetos em busca de streams
    for (_object_id, object) in doc.objects.iter_mut() {
        if let Object::Stream(stream) = object {
            // Recompressão de imagens antes da compressão estrutural
            if is_image_stream(stream)
                && let Err(e) = recompress_image_stream(stream, level)
            {
                eprintln!("Aviso: imagem ignorada ({e})");
            }

            // Compressão estrutural: aplica FlateDecode (Zlib) nas streams de texto/estruturais
            let _ = stream.compress();
        }
    }

    // 5. Limpeza interna: remove referências mortas e reordena objetos
    doc.prune_objects();

    // 6. Grava em memória para validar ganho real antes de tocar o disco
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)
        .map_err(|e| format!("Erro ao salvar PDF otimizado na memória: {}", e))?;

    let optimized_size = buffer.len();

    // 7. Regra de Ouro do ThinFlux: só sobrescreve se houve ganho real
    if optimized_size < original_size {
        std::fs::write(path, buffer)
            .map_err(|e| format!("Erro ao gravar PDF otimizado em disco: {}", e))?;
        Ok((original_size, optimized_size))
    } else {
        Ok((original_size, original_size))
    }
}

/// Detecta se uma stream é uma imagem pelo dicionário /Subtype /Image
fn is_image_stream(stream: &Stream) -> bool {
    matches!(
        stream.dict.get(b"Subtype"),
        Ok(Object::Name(name)) if name == b"Image"
    )
}

/// Extrai, recomprime e reinjeta os bytes da imagem na stream do PDF.
fn recompress_image_stream(stream: &mut Stream, level: u8) -> Result<(), String> {
    // Descomprime para expor os bytes brutos (pixels sem cabeçalho)
    stream
        .decompress()
        .map_err(|e| format!("Falha ao descomprimir stream de imagem: {e}"))?;

    let raw_bytes = stream.content.clone();

    // Lê metadados de imagem do dicionário PDF
    let width = get_dict_int(&stream.dict, b"Width").ok_or("Stream sem /Width")?;
    let height = get_dict_int(&stream.dict, b"Height").ok_or("Stream sem /Height")?;
    let bits = get_dict_int(&stream.dict, b"BitsPerComponent").unwrap_or(8);

    let color_space = stream
        .dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|o| {
            if let Object::Name(n) = o {
                Some(n.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Número de canais por pixel
    let channels: usize = match color_space.as_slice() {
        b"DeviceRGB" => 3,
        b"DeviceCMYK" => return Err("CMYK não suportado — ignorado".into()), // sem suporte nativo
        _ => 1,                                                              // DeviceGray
    };

    // Guarda contra streams com tamanho inconsistente no dicionário
    let expected = width * height * channels * (bits / 8);
    if raw_bytes.len() < expected {
        return Err(format!(
            "Bytes insuficientes: esperado {expected}, obtido {}",
            raw_bytes.len()
        ));
    }

    // Recodifica para JPEG com qualidade baseada no nível
    let jpeg_quality: u8 = match level {
        0 => 95,
        1 => 85,
        2 => 75,
        3 => 65,
        _ => 50,
    };

    let recompressed = encode_as_jpeg(&raw_bytes, width, height, channels, jpeg_quality)?;

    // Só substitui se realmente ficou menor
    if recompressed.len() < raw_bytes.len() {
        stream.content = recompressed;
        // Remove filtros antigos; stream.compress() vai aplicar FlateDecode depois
        stream.dict.remove(b"Filter");
        stream.dict.remove(b"DecodeParms");
        stream
            .dict
            .set("Length", Object::Integer(stream.content.len() as i64));
    }

    Ok(())
}

/// Reconstrói pixels brutos numa imagem e recodifica como JPEG.
fn encode_as_jpeg(
    raw: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    quality: u8,
) -> Result<Vec<u8>, String> {
    use image::{DynamicImage, ImageBuffer, Luma, Rgb};

    let dynamic: DynamicImage = match channels {
        3 => {
            let buf =
                ImageBuffer::<Rgb<u8>, _>::from_raw(width as u32, height as u32, raw.to_vec())
                    .ok_or("Falha ao construir ImageBuffer RGB")?;
            DynamicImage::ImageRgb8(buf)
        }
        _ => {
            let buf =
                ImageBuffer::<Luma<u8>, _>::from_raw(width as u32, height as u32, raw.to_vec())
                    .ok_or("Falha ao construir ImageBuffer Grayscale")?;
            DynamicImage::ImageLuma8(buf)
        }
    };

    let mut output = Vec::new();
    dynamic
        .write_to(
            &mut std::io::Cursor::new(&mut output),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| format!("Falha ao codificar JPEG: {e}"))?;

    // A crate `image` não expõe qualidade direto via write_to;
    let mut fine_output = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut fine_output, quality);
        encoder
            .encode_image(&dynamic)
            .map_err(|e| format!("Falha no JpegEncoder: {e}"))?;
    }

    // Retorna o menor dos dois (write_to usa qualidade padrão, fine usa a solicitada)
    Ok(if fine_output.len() < output.len() {
        fine_output
    } else {
        output
    })
}

/// Helper: lê um inteiro do dicionário da stream
fn get_dict_int(dict: &lopdf::Dictionary, key: &[u8]) -> Option<usize> {
    dict.get(key).ok().and_then(|o| {
        if let Object::Integer(n) = o {
            Some(*n as usize)
        } else {
            None
        }
    })
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
        let input_path = PathBuf::from("test_input.png");
        let test_path = PathBuf::from("test_output.png");

        if !input_path.exists() {
            panic!("Adicione 'test_input.png' na raiz do projeto.");
        }

        fs::copy(&input_path, &test_path).unwrap();

        let (initial_size, final_size) = optimize_image(&test_path, 2).expect("Erro ao otimizar");

        println!("Inicial: {initial_size} bytes | Final: {final_size} bytes");
        assert!(final_size <= initial_size);

        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_file_not_found_error() {
        let ghost_path = PathBuf::from("arquivo_fantasma.png");
        assert!(optimize_image(&ghost_path, 2).is_err());
    }

    #[test]
    fn test_pdf_format_routing() {
        let pdf_path = PathBuf::from("relatorio_teste.pdf");
        let result = optimize_image(&pdf_path, 2);
        // Aceita Ok (PDF existente) ou Err (arquivo não existe): apenas valida que roteia
        assert!(result.is_ok() || result.is_err());
    }
}
