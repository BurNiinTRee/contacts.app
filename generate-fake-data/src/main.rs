use anyhow::Result;
use fake::{locales::EN, Fake};
use rand::{Rng, SeedableRng};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Write},
};

fn main() -> Result<()> {
    let num: i64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(9_000);

    let rng = rand::rngs::SmallRng::from_seed(*b"A cool seed that's 32 bytes long");

    match std::env::args().nth(2) {
        Some(path) => {
            let file = File::create(path)?;
            let out = BufWriter::new(file);
            write(out, num, rng)?;
        }
        None => {
            let out = std::io::stdout().lock();
            write(out, num, rng)?;
        }
    }
    Ok(())
}

fn write<B: Write, R: Rng>(mut out: B, num: i64, mut rng: R) -> Result<()> {
    write!(
        out,
        "INSERT INTO Contacts (first, last, phone, email) VALUES "
    )?;

    let mut used_emails = HashSet::new();

    'outer: for n in 0..num {
        eprintln!("generating {n}th entry");
        let mut first: &'static str = fake::faker::name::raw::FirstName(EN).fake_with_rng(&mut rng);
        let mut last: &'static str = fake::faker::name::raw::LastName(EN).fake_with_rng(&mut rng);
        let phone: String = fake::faker::phone_number::raw::PhoneNumber(EN).fake_with_rng(&mut rng);
        let mut email: String = format!("{first}.{last}@example.com");
        let mut tries = 1;
        while used_emails.contains(&email) {
            first = fake::faker::name::raw::FirstName(EN).fake_with_rng(&mut rng);
            last = fake::faker::name::raw::LastName(EN).fake_with_rng(&mut rng);
            email = format!("{first}.{last}@example.com");
            tries += 1;
            if tries % 1000 == 0 {
                eprintln!("tried {tries} times");
            }
            if tries == 10_000_000 {
                break 'outer;
            }
        }

        writeln!(
            out,
            r#"('{}', '{}', '{}', '{}'),"#,
            postgres_escape(first),
            postgres_escape(last),
            postgres_escape(&phone),
            postgres_escape(&email)
        )?;
        used_emails.insert(email);
    }

    write!(out, ";")?;
    Ok(())
}

fn postgres_escape(input: &str) -> String {
    input.replace('\'', "\'\'")
}
