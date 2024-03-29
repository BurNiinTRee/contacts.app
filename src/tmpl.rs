use super::paths;
use askama::Template;

#[derive(Default)]
pub struct Layout {
    flashes: Vec<(axum_flash::Level, String)>,
}

#[derive(Template)]
#[template(path = "contacts.html")]
pub struct Contacts {
    pub layout: Layout,
    pub search_term: Option<String>,
    pub contacts: Vec<Contact>,
}

#[derive(Default, Template)]
#[template(path = "new-contact.html")]
pub struct NewContact {
    pub layout: Layout,
    contact: Contact,
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
    pub first: Option<String>,
    pub last: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Template)]
#[template(path = "new-contact.html", block = "contact_fields")]
pub struct ContactFields {
    contact: Contact,
}

impl ContactFields {
    fn new(contact: &Contact) -> Self {
        Self {
            contact: contact.clone(),
        }
    }
}
