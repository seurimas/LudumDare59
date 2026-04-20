use LudumDare59::dictionary;

fn main() {
    let word = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: futhark_it <word>");
        std::process::exit(1);
    });

    match dictionary::futharkation_from_word(&word.to_lowercase()) {
        Ok(f) => println!("{} -> {}", f.word, f.letters),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
