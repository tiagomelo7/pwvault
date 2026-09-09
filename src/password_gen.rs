use rand::{rngs::OsRng, Rng};

pub struct GeneratorOptions {
    pub length: usize,
    pub use_uppercase: bool,
    pub use_lowercase: bool,
    pub use_digits: bool,
    pub use_symbols: bool,
}

pub fn generate_password(options: &GeneratorOptions) -> String {
    let maiusculas = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let minusculas = "abcdefghijklmnopqrstuvwxyz";
    let digitos = "0123456789";
    let simbolos = "!@#$%^&*()_+-=[]{}";

    let mut characters = String::new();

    if options.use_uppercase {
        characters.push_str(maiusculas);
    }
    if options.use_lowercase {
        characters.push_str(minusculas);
    }
    if options.use_digits {
        characters.push_str(digitos);
    }
    if options.use_symbols {
        characters.push_str(simbolos);
    }
    if characters.is_empty() {
        return String::new();
    }

    let mut password = String::with_capacity(options.length);
    let mut rng = OsRng;

    for _ in 0..options.length {
        let index = rng.gen_range(0..characters.len());

        let character = characters.chars().nth(index).unwrap();

        password.push(character);
    }

    password
}