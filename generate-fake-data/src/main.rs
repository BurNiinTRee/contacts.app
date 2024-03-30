use anyhow::Result;
use fake::{locales::EN, Fake};
use std::{collections::HashSet, io::Write};

fn main() -> Result<()> {
    let num: i64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(9_000);

    let mut out = std::io::stdout().lock();
    write!(
        out,
        "INSERT INTO Contacts (first, last, phone, email) VALUES "
    )?;

    let mut used_emails = HashSet::new();

    for n in 0..num {
        eprintln!("generating {n}th entry");
        let first: &'static str = fake::faker::name::raw::FirstName(EN).fake();
        let last: &'static str = fake::faker::name::raw::LastName(EN).fake();
        let phone: String = fake::faker::phone_number::raw::PhoneNumber(EN).fake();
        let mut email: String = fake::faker::internet::raw::SafeEmail(EN).fake();
        let mut tries = 1;
        while used_emails.contains(&email) {
            email = fake::faker::internet::raw::SafeEmail(EN).fake();
            tries += 1;
            if tries % 1000 == 0 {
                eprintln!("tried {tries} times");
            }
        }

        write!(
            out,
            r#"("{}", "{}", "{}", "{}"),"#,
            first, last, phone, email
        )?;
        used_emails.insert(email);
    }

    write!(out, ";")?;
    Ok(())
}