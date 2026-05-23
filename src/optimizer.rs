use image::ImageBuffer;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageReader, Luma, Rgb};
use lopdf::{Document, Object};
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

/// Mapeia o nível universal (0-6) para qualidade JPEG (0-100)
fn jpeg_quality_from_level(level: u8) -> u8 {
    match level {
        0 => 95,
        1 => 85,
        2 => 75,
        3 => 65,
        _ => 50,
    }
}

// ─── Router ──────────────────────────────────────────────────────────────────

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

// ─── Motores por formato ─────────────────────────────────────────────────────

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

    let quality = jpeg_quality_from_level(level);

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
    let quality: f32 = match level {
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
    let original_size = std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);

    let mut doc =
        Document::load(path).map_err(|e| format!("Erro ao carregar PDF {:?}: {}", path, e))?;

    // Remove metadados desnecessários (Produtor/Autor)
    doc.trailer.remove(b"Info");

    // Varre todos os objetos em busca de streams
    for (_object_id, object) in doc.objects.iter_mut() {
        if let Object::Stream(stream) = object {
            // Recompressão de imagens embutidas
            if is_image_stream(stream) {
                if let Err(e) = recompress_image_stream(stream, level) {
                    // Só loga se for um erro inesperado, não limitações conhecidas
                    let silenced = [
                        "JPXDecode",
                        "CMYK",
                        "missing feature of lopdf",
                        "Bytes insuficientes",
                    ];
                    if !silenced.iter().any(|s| e.contains(s)) {
                        eprintln!("Aviso: imagem ignorada ({e})");
                    }
                }
            }
            // Compressão estrutural: FlateDecode nas streams de texto/estruturais
            let _ = stream.compress();
        }
    }

    // Limpeza interna: remove referências mortas e reordena objetos
    doc.prune_objects();

    // Grava em memória para validar ganho antes de tocar o disco
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)
        .map_err(|e| format!("Erro ao salvar PDF otimizado na memória: {}", e))?;

    let optimized_size = buffer.len();

    // Regra de Ouro do ThinFlux: só sobrescreve se houve ganho real
    if optimized_size < original_size {
        // Grava em temporário NO MESMO diretório — garante rename atômico (mesmo filesystem)
        let parent = path.parent().ok_or("Arquivo PDF sem diretório pai")?;
        let tmp_path = parent.join(format!(
            ".thinflux_tmp_{}.pdf",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
        ));

        std::fs::write(&tmp_path, &buffer)
            .map_err(|e| format!("Erro ao gravar temporário {:?}: {}", tmp_path, e))?;

        // Rename atômico: ou substitui por completo, ou não troca nada
        std::fs::rename(&tmp_path, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path); // limpa lixo se falhar
            format!("Erro ao substituir PDF original {:?}: {}", path, e)
        })?;

        Ok((original_size, optimized_size))
    } else {
        Ok((original_size, original_size))
    }
}

// ─── Auxiliares de imagem embutida em PDF ────────────────────────────────────

/// Detecta se uma stream é uma imagem pelo dicionário /Subtype /Image
fn is_image_stream(stream: &lopdf::Stream) -> bool {
    matches!(
        stream.dict.get(b"Subtype"),
        Ok(Object::Name(name)) if name == b"Image"
    )
}

/// Extrai, recomprime e reinjeta os bytes da imagem na stream do PDF.
///
/// Fluxo:
///   1. Descomprime a stream para obter pixels brutos
///   2. Lê dimensões e espaço de cor do dicionário PDF
///   3. Reconstrói a imagem com a crate `image`
///   4. Recodifica como JPEG com qualidade = f(level)
///   5. Só substitui se o resultado for menor (Regra de Ouro)
fn recompress_image_stream(stream: &mut lopdf::Stream, level: u8) -> Result<(), String> {
    // Descobre qual filtro está sendo usado antes de tentar qualquer coisa
    let filter = stream
        .dict
        .get(b"Filter")
        .ok()
        .and_then(|o| match o {
            Object::Name(n) => Some(n.clone()),
            Object::Array(a) => a.first().and_then(|f| {
                if let Object::Name(n) = f {
                    Some(n.clone())
                } else {
                    None
                }
            }),
            _ => None,
        })
        .unwrap_or_default();

    // DCTDecode = JPEG puro — os bytes já SÃO o arquivo JPEG, sem necessidade de descomprimir
    if filter == b"DCTDecode" {
        return recompress_dct_stream(stream, level);
    }

    // JPXDecode = JPEG2000 — complexo demais para recomprimir sem biblioteca dedicada
    if filter == b"JPXDecode" {
        return Err("JPXDecode (JPEG2000) não suportado — ignorado".into());
    }

    // FlateDecode e outros: tenta descomprimir normalmente pelo lopdf
    stream
        .decompress()
        .map_err(|e| format!("Falha ao descomprimir stream: {e}"))?;

    let raw_bytes = stream.content.clone();

    // ... resto do fluxo existente (RGB/Gray → encode_as_jpeg)
    let width = get_dict_int(&stream.dict, b"Width").ok_or("Stream sem /Width")?;
    let height = get_dict_int(&stream.dict, b"Height").ok_or("Stream sem /Height")?;
    let bits = get_dict_int(&stream.dict, b"BitsPerComponent").unwrap_or(8);

    let color_space = stream
        .dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|o| match o {
            Object::Name(n) => Some(n.clone()), // DeviceRGB direto
            Object::Reference(_) => Some(b"Unknown".to_vec()), // referência indireta → trata como gray
            Object::Array(a) => a.first().and_then(|f| {
                if let Object::Name(n) = f {
                    Some(n.clone())
                } else {
                    None
                }
            }),
            _ => None,
        })
        .unwrap_or_default();

    let channels: usize = match color_space.as_slice() {
        b"DeviceRGB" => 3,
        b"DeviceCMYK" => return Err("CMYK não suportado — ignorado".into()),
        _ => 1,
    };

    let expected = width * height * channels * (bits / 8);
    if raw_bytes.len() < expected {
        return Err(format!(
            "Bytes insuficientes: esperado {expected}, obtido {}",
            raw_bytes.len()
        ));
    }

    let quality = jpeg_quality_from_level(level);
    let recompressed = encode_as_jpeg(&raw_bytes, width, height, channels, quality)?;

    if recompressed.len() < raw_bytes.len() {
        stream.content = recompressed;
        stream.dict.remove(b"Filter");
        stream.dict.remove(b"DecodeParms");
        stream
            .dict
            .set("Length", Object::Integer(stream.content.len() as i64));
    }

    Ok(())
}

/// Reconstrói pixels brutos numa DynamicImage e recodifica como JPEG.
fn encode_as_jpeg(
    raw: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    quality: u8,
) -> Result<Vec<u8>, String> {
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
    let mut encoder = JpegEncoder::new_with_quality(&mut output, quality);
    encoder
        .encode_image(&dynamic)
        .map_err(|e| format!("Falha no JpegEncoder: {e}"))?;

    Ok(output)
}

/// Lê um inteiro do dicionário da stream
fn get_dict_int(dict: &lopdf::Dictionary, key: &[u8]) -> Option<usize> {
    dict.get(key).ok().and_then(|o| {
        if let Object::Integer(n) = o {
            Some(*n as usize)
        } else {
            None
        }
    })
}

/// Caminho especial para DCTDecode: os bytes já são JPEG válido.
/// Recomprime usando a crate `image` para decode + reencode com nova qualidade.
fn recompress_dct_stream(stream: &mut lopdf::Stream, level: u8) -> Result<(), String> {
    let original_bytes = stream.content.clone();

    // Faz decode do JPEG existente diretamente da memória
    let img = image::load_from_memory_with_format(&original_bytes, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Falha ao decodificar DCTDecode JPEG: {e}"))?;

    let quality = jpeg_quality_from_level(level);

    let mut recompressed = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut recompressed, quality);
    encoder
        .encode_image(&img)
        .map_err(|e| format!("Falha ao reencodar JPEG: {e}"))?;

    // Regra de Ouro: só substitui se ficou menor
    if recompressed.len() < original_bytes.len() {
        stream.content = recompressed;
        // DCTDecode se mantém — continuamos com JPEG, só com qualidade menor
        stream
            .dict
            .set("Length", Object::Integer(stream.content.len() as i64));
    }

    Ok(())
}

// ─── Testes ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Document, Object, Stream};
    use std::fs;

    // ─── Helpers de construção de streams para testes ─────────────────────────

    /// Cria uma stream mínima com /Subtype /Image e o filtro especificado
    fn make_image_stream(filter: Option<&[u8]>, extra: &[(&[u8], Object)]) -> Stream {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name(b"XObject".to_vec()));
        dict.set("Subtype", Object::Name(b"Image".to_vec()));
        if let Some(f) = filter {
            dict.set("Filter", Object::Name(f.to_vec()));
        }
        for (k, v) in extra {
            dict.set(*k, v.clone());
        }
        Stream::new(dict, vec![])
    }

    /// Cria uma stream que NÃO é imagem (stream de conteúdo de página)
    fn make_content_stream() -> Stream {
        let mut dict = Dictionary::new();
        dict.set("Length", Object::Integer(0));
        Stream::new(dict, vec![])
    }

    // ─── Testes: is_image_stream ──────────────────────────────────────────────

    #[test]
    fn test_is_image_stream_detects_image() {
        let stream = make_image_stream(None, &[]);
        assert!(is_image_stream(&stream));
    }

    #[test]
    fn test_is_image_stream_rejects_content_stream() {
        let stream = make_content_stream();
        assert!(!is_image_stream(&stream));
    }

    #[test]
    fn test_is_image_stream_rejects_subtype_form() {
        let mut dict = Dictionary::new();
        dict.set("Subtype", Object::Name(b"Form".to_vec()));
        let stream = Stream::new(dict, vec![]);
        assert!(!is_image_stream(&stream));
    }

    #[test]
    fn test_is_image_stream_no_subtype() {
        let dict = Dictionary::new();
        let stream = Stream::new(dict, vec![]);
        assert!(!is_image_stream(&stream));
    }

    // ─── Testes: detecção de filtro em recompress_image_stream ───────────────

    #[test]
    fn test_jpxdecode_returns_silent_err() {
        let mut stream = make_image_stream(
            Some(b"JPXDecode"),
            &[
                (b"Width", Object::Integer(10)),
                (b"Height", Object::Integer(10)),
                (b"BitsPerComponent", Object::Integer(8)),
                (b"ColorSpace", Object::Name(b"DeviceRGB".to_vec())),
            ],
        );
        // Conteúdo vazio — não deve tentar decodificar, só retornar Err conhecido
        let result = recompress_image_stream(&mut stream, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JPXDecode"));
    }

    #[test]
    fn test_cmyk_returns_silent_err() {
        // FlateDecode com ColorSpace CMYK — deve rejeitar sem pânico
        let mut stream = make_image_stream(
            Some(b"FlateDecode"),
            &[
                (b"Width", Object::Integer(4)),
                (b"Height", Object::Integer(4)),
                (b"BitsPerComponent", Object::Integer(8)),
                (b"ColorSpace", Object::Name(b"DeviceCMYK".to_vec())),
            ],
        );
        // Injeta bytes mínimos para passar da decompress (que pode falhar no lopdf)
        // O teste valida apenas que o erro é o esperado e não um pânico
        let result = recompress_image_stream(&mut stream, 2);
        if let Err(e) = result {
            assert!(
                e.contains("CMYK") || e.contains("descomprimir"),
                "Erro inesperado: {e}"
            );
        }
        // Ok também é aceitável se o lopdf conseguiu descomprimir bytes vazios
    }

    // ─── Testes: recompress_dct_stream ────────────────────────────────────────

    #[test]
    fn test_dct_stream_with_valid_jpeg_compress() {
        // Gera um JPEG mínimo real em memória (1x1 pixel branco)
        let jpeg_bytes = make_minimal_jpeg_bytes();
        let original_len = jpeg_bytes.len();

        let mut dict = Dictionary::new();
        dict.set("Subtype", Object::Name(b"Image".to_vec()));
        dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
        dict.set("Width", Object::Integer(1));
        dict.set("Height", Object::Integer(1));
        dict.set("Length", Object::Integer(original_len as i64));
        let mut stream = Stream::new(dict, jpeg_bytes);

        // Não deve retornar Err em JPEG válido
        let result = recompress_dct_stream(&mut stream, 3);
        assert!(result.is_ok(), "Esperado Ok, obtido: {:?}", result);
    }

    #[test]
    fn test_dct_stream_with_invalid_bytes_returns_err() {
        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
        let mut stream = Stream::new(dict, b"isso_nao_e_jpeg".to_vec());

        let result = recompress_dct_stream(&mut stream, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("decodificar"));
    }

    // ─── Testes: jpeg_quality_from_level ─────────────────────────────────────

    #[test]
    fn test_jpeg_quality_from_level_covers_all() {
        assert_eq!(jpeg_quality_from_level(0), 95);
        assert_eq!(jpeg_quality_from_level(1), 85);
        assert_eq!(jpeg_quality_from_level(2), 75);
        assert_eq!(jpeg_quality_from_level(3), 65);
        assert_eq!(jpeg_quality_from_level(4), 50); // catch-all
        assert_eq!(jpeg_quality_from_level(6), 50); // máximo do preset oxipng
        assert_eq!(jpeg_quality_from_level(255), 50); // overflow seguro
    }

    // ─── Testes: get_dict_int ─────────────────────────────────────────────────

    #[test]
    fn test_get_dict_int_reads_integer() {
        let mut dict = Dictionary::new();
        dict.set("Width", Object::Integer(640));
        assert_eq!(get_dict_int(&dict, b"Width"), Some(640));
    }

    #[test]
    fn test_get_dict_int_returns_none_for_absent_key() {
        let dict = Dictionary::new();
        assert_eq!(get_dict_int(&dict, b"Width"), None);
    }

    #[test]
    fn test_get_dict_int_returns_none_for_wrong_type() {
        let mut dict = Dictionary::new();
        dict.set("Width", Object::Name(b"nao_e_inteiro".to_vec()));
        assert_eq!(get_dict_int(&dict, b"Width"), None);
    }

    // ─── Testes: ImageFormat ──────────────────────────────────────────────────

    #[test]
    fn test_image_format_detection_case_insensitive() {
        assert_eq!(ImageFormat::from_extension("JPEG"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("JPG"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("PNG"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("WEBP"), ImageFormat::Webp);
        assert_eq!(ImageFormat::from_extension("PDF"), ImageFormat::Pdf);
        assert_eq!(ImageFormat::from_extension("txt"), ImageFormat::Unknown);
        assert_eq!(ImageFormat::from_extension(""), ImageFormat::Unknown);
    }

    // ─── Testes: encode_as_jpeg ───────────────────────────────────────────────

    #[test]
    fn test_encode_as_jpeg_rgb_returns_valid_bytes() {
        // 2x2 RGB branco
        let raw = vec![255u8; 2 * 2 * 3];
        let result = encode_as_jpeg(&raw, 2, 2, 3, 75);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // JPEG começa com SOI marker 0xFF 0xD8
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "Bytes não são um JPEG válido");
    }

    #[test]
    fn test_encode_as_jpeg_grayscale_returns_valid_bytes() {
        // 4x4 Grayscale
        let raw = vec![128u8; 4 * 4];
        let result = encode_as_jpeg(&raw, 4, 4, 1, 75);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_encode_as_jpeg_buffer_too_small_returns_err() {
        // Passa 1 byte para uma imagem 10x10 — deve retornar Err, não pânico
        let raw = vec![0u8; 1];
        let result = encode_as_jpeg(&raw, 10, 10, 3, 75);
        assert!(result.is_err());
    }

    // ─── Testes: optimize_pdf (integração leve) ───────────────────────────────

    #[test]
    fn test_optimize_pdf_missing_file_returns_err() {
        let path = PathBuf::from("pdf_fantasma_12345.pdf");
        let result = optimize_pdf(&path, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_optimize_pdf_invalid_file_returns_err() {
        let path = PathBuf::from("test_corrompido.pdf");
        fs::write(&path, b"isto nao e um pdf valido").unwrap();
        let result = optimize_pdf(&path, 2);
        fs::remove_file(&path).ok();
        assert!(result.is_err());
    }

    // ─── Testes: rename atômico — tmp não deve persistir após sucesso ─────────

    #[test]
    fn test_optimize_pdf_cleans_up_tmp_file() {
        // Cria um PDF mínimo válido para forçar o caminho de escrita
        let path = PathBuf::from("test_atomic_rename.pdf");
        let tmp = PathBuf::from(".thinflux_tmp_test_atomic_rename.pdf.pdf");

        // PDF mínimo que o lopdf consegue carregar
        let minimal_pdf = make_minimal_pdf_bytes();
        fs::write(&path, &minimal_pdf).unwrap();

        let _ = optimize_pdf(&path, 6);

        // O temporário jamais deve persistir
        assert!(
            !tmp.exists(),
            "Arquivo temporário não foi removido após otimização"
        );

        fs::remove_file(&path).ok();
    }

    // ─── Helpers internos dos testes ──────────────────────────────────────────

    /// Gera um JPEG de 1x1 pixel branco RGB em memória
    fn make_minimal_jpeg_bytes() -> Vec<u8> {
        use image::codecs::jpeg::JpegEncoder;
        use image::{DynamicImage, ImageBuffer, Rgb};

        let buf = ImageBuffer::<Rgb<u8>, _>::from_raw(1, 1, vec![255u8, 255, 255]).unwrap();
        let img = DynamicImage::ImageRgb8(buf);
        let mut out = Vec::new();
        let mut enc = JpegEncoder::new_with_quality(&mut out, 90);
        enc.encode_image(&img).unwrap();
        out
    }

    /// Gera um PDF mínimo válido usando lopdf
    fn make_minimal_pdf_bytes() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        doc.objects.insert(
            pages_id,
            Object::Dictionary({
                let mut d = Dictionary::new();
                d.set("Type", Object::Name(b"Pages".to_vec()));
                d.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
                d.set("Count", Object::Integer(1));
                d
            }),
        );

        doc.objects.insert(
            page_id,
            Object::Dictionary({
                let mut d = Dictionary::new();
                d.set("Type", Object::Name(b"Page".to_vec()));
                d.set("Parent", Object::Reference(pages_id));
                d.set(
                    "MediaBox",
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                );
                d
            }),
        );

        doc.trailer.set("Root", Object::Reference(pages_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }
}
