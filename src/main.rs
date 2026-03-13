mod scanner;

use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("./assets");

    let files = scanner::find_png_files(path);

    println!("Arquivos encontrados: {:?}", files);
}
