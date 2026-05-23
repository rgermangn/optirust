use assert_cmd::Command;
use lopdf::{Dictionary, Document, Object, Stream};
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// Helper que cria um ecossistema completo de mídias para testes de integração
fn setup_test_environment() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().to_path_buf();

    // 1. Cria um PNG válido fake
    let png_path = path.join("teste.png");
    let img = image::DynamicImage::new_rgb8(10, 10);
    img.save_with_format(&png_path, image::ImageFormat::Png)
        .unwrap();

    // 2. Cria os bytes de um JPEG na memória para embutir no PDF
    let mut jpeg_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg).unwrap();

    // 3. Cria um arquivo WebP válido real (usando o encoder nativo para o teste)
    let webp_path = path.join("teste.webp");
    img.save_with_format(&webp_path, image::ImageFormat::WebP)
        .unwrap();

    // 4. Cria uma Stream de imagem válida padrão PDF
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"XObject".to_vec()));
    dict.set("Subtype", Object::Name(b"Image".to_vec()));
    dict.set("Width", Object::Integer(10));
    dict.set("Height", Object::Integer(10));
    dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    dict.set("BitsPerComponent", Object::Integer(8));

    let image_stream = Stream::new(dict, jpeg_bytes);

    // Cria a árvore estrutural básica mínima de um documento PDF
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let stream_obj = Object::Stream(image_stream);
    let image_id = doc.add_object(stream_obj);

    let mut resources = Dictionary::new();
    let mut xobject = Dictionary::new();
    xobject.set("Im1", Object::Reference(image_id));
    resources.set("XObject", Object::Dictionary(xobject));

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set("Parent", Object::Reference(pages_id));
    page_dict.set("Resources", Object::Dictionary(resources));
    doc.set_object(page_id, Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    doc.set_object(pages_id, Object::Dictionary(pages_dict));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));

    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Salva o PDF gerado na pasta temporária
    let pdf_path = path.join("documento_pesado.pdf");
    doc.save(&pdf_path).unwrap();

    (dir, path)
}

#[test]
fn test_cli_integration_success_with_filters() {
    let (_dir, path) = setup_test_environment();
    let mut cmd = Command::cargo_bin("thinflux").unwrap();

    // Executa focando estritamente em PNG
    let assert = cmd
        .arg("run")
        .arg(&path)
        .arg("--level")
        .arg("2")
        .arg("--types")
        .arg("png")
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("Otimizando em nível: 2"))
        .stdout(predicate::str::contains("Encontrados: 1 arquivos."));
}

#[test]
fn test_cli_webp_optimization_integration() {
    let (_dir, path) = setup_test_environment();

    let webp_path = path.join("teste.webp");
    let size_before = fs::metadata(&webp_path).unwrap().len();

    let mut cmd = Command::cargo_bin("thinflux").unwrap();

    // Executa: thinflux run <pasta> --level 4 --types webp
    let assert = cmd
        .arg("run")
        .arg(&path)
        .arg("--level")
        .arg("4")
        .arg("--types")
        .arg("webp")
        .assert();

    // VALIDAÇÕES:
    assert
        .success()
        .stdout(predicate::str::contains("Otimizando em nível: 4"))
        .stdout(predicate::str::contains("Encontrados: 1 arquivos."));

    let size_after = fs::metadata(&webp_path).unwrap().len();
    assert!(
        size_after <= size_before,
        "O WebP deveria ter sido otimizado ou mantido estável em bytes"
    );

    // Valida se o WebP gerado continua íntegro e decodificável
    let decoded = image::open(&webp_path);
    assert!(
        decoded.is_ok(),
        "O motor de WebP corrompeu o arquivo final!"
    );
}

#[test]
fn test_cli_pdf_optimization_integration() {
    let (_dir, path) = setup_test_environment();
    let pdf_path = path.join("documento_pesado.pdf");
    let size_before = fs::metadata(&pdf_path).unwrap().len();

    let mut cmd = Command::cargo_bin("thinflux").unwrap();

    let assert = cmd
        .arg("run")
        .arg(&path)
        .arg("--level")
        .arg("5")
        .arg("--types")
        .arg("pdf")
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("Otimizando em nível: 5"))
        .stdout(predicate::str::contains("Encontrados: 1 arquivos."));

    let size_after = fs::metadata(&pdf_path).unwrap().len();
    assert!(
        size_after <= size_before,
        "O motor deveria ter reduzido ou mantido o tamanho estável do PDF"
    );

    let re_opened_doc = Document::load(&pdf_path);
    assert!(
        re_opened_doc.is_ok(),
        "O PDF final foi corrompido pelo motor de otimização!"
    );
}

#[test]
fn test_cli_level_validation_bounds() {
    let (_dir, path) = setup_test_environment();
    let mut cmd = Command::cargo_bin("thinflux").unwrap();

    let assert = cmd.arg("run").arg(&path).arg("--level").arg("8").assert();

    assert.failure().stderr(predicate::str::contains(
        "invalid value '8' for '--level <0-6>'",
    ));
}

#[test]
fn test_cli_silent_mode_suppression() {
    let (_dir, path) = setup_test_environment();
    let mut cmd = Command::cargo_bin("thinflux").unwrap();

    let assert = cmd.arg("run").arg(&path).arg("--silent").assert();

    assert.success().stdout(predicate::str::is_empty());
}
