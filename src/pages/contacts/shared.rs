use askama::Template;

#[derive(Template)]
#[template(path = "contact-fields.html")]
pub struct ContactFields {
    pub contact: Contact,
}

impl ContactFields {
    pub fn new(contact: &Contact) -> Self {
        Self {
            contact: contact.clone(),
        }
    }
}

pub struct Layout {
    pub flashes: Option<axum_flash::IncomingFlashes>,
}

impl Layout {
    pub fn flashes<'a>(&'a self) -> Box<dyn Iterator<Item = (axum_flash::Level, &str)> + 'a> {
        match &self.flashes {
            Some(flashes) => Box::new(flashes.into_iter()),
            None => Box::new(None.into_iter()),
        }
    }
}

#[derive(Default, Clone)]
pub struct Contact {
    pub id: i64,
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
    pub errors: ContactFieldErrors,
}
#[derive(Clone, Default)]
pub struct ContactFieldErrors {
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}
