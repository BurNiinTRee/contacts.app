use super::paths;
use askama::Template;

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

#[derive(Template)]
#[template(path = "contacts.html")]
pub struct Contacts {
    pub layout: Layout,
    pub search_term: Option<String>,
    pub page: u64,
    pub contacts: Vec<Contact>,
}

#[derive(Template)]
#[template(path = "new-contact.html")]
pub struct NewContact {
    pub layout: Layout,
    pub contact: Contact,
}

#[derive(Template)]
#[template(path = "view-contact.html")]
pub struct ViewContact {
    pub layout: Layout,
    pub contact: Contact,
}
#[derive(Template)]
#[template(path = "edit-contact.html")]
pub struct EditContact {
    pub layout: Layout,
    pub contact: Contact,
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

#[derive(Template)]
#[template(path = "new-contact.html", block = "contact_fields")]
pub struct ContactFields {
    pub contact: Contact,
}

impl ContactFields {
    fn new(contact: &Contact) -> Self {
        Self {
            contact: contact.clone(),
        }
    }
}

#[derive(Clone, Default)]
pub struct ContactFieldErrors {
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}
